# IBM 1130 ABI

Status: this ABI is **anchored on the historical 1130 calling
conventions** documented in IBM's 1130 Subroutine Library
(C26-5929-4), the FORTRAN compiler's runtime, and the standard CALL /
LIBF linkage idioms. The 1130 manuals do not codify a single official
ABI -- different software ecosystems (FORTRAN, Assembler, Disk
Monitor System) settled on different conventions on top of the BSI
instruction -- but the cross-cutting invariants that all of them
respected are load-bearing for our codegen, and we follow them.

A first-pass invented ABI was committed in saga step 8 and replaced
by this version after research against bitsavers listings. See
`gen-isa/docs/abi-linkage.md` for the research notes that drove the
revision.

ASCII-only by convention.

References:

- IBM 1130 Subroutine Library, C26-5929-4 (1966); especially pp. v
  (Introduction), 2 ("ISS Operation"), 6 ("Basic ISS Calling
  Sequence"), 9 (core-storage map).
- IBM 1130 FORTRAN Programming Techniques, C20-1642-0.
- `gen-isa/docs/abi-linkage.md` for the research synthesis.
- `gen-isa/docs/decisions.md` Sec 1 (saga directional decisions).
- `gen-isa/docs/porting-guide.md` Sec 5 (ABI invention guidance).
- IBM 1130 Functional Characteristics (GA26-5881) for hardware
  register state and instruction semantics.

## 1. Machine state recap

The 1130 has six addressable registers from the toolchain's point of
view (see `sw-ibm1130-isa::register::Reg`):

| Reg  | Width | Hardware role                                  |
| ---- | ----- | ---------------------------------------------- |
| ACC  | 16    | accumulator                                    |
| EXT  | 16    | extension; pairs with ACC for 32-bit ops       |
| XR1  | 16    | index register 1 (memory-mapped at addr 1)     |
| XR2  | 16    | index register 2 (memory-mapped at addr 2)     |
| XR3  | 16    | index register 3 (memory-mapped at addr 3)     |
| IAR  | 16    | instruction address register (program counter) |

There is no hardware stack pointer, no hardware push/pop, no link
register. Subroutine calls use BSI (Branch and Store IAR), which
writes the return address into the word at the call target and jumps
to target+1. Every linkage idiom on the 1130 (CALL, LIBF, ISS) is a
shape on top of BSI.

Memory is word-addressed: 1 address-unit = 1 16-bit word = 2 bytes.
Word alignment is the only alignment that exists.

## 2. Register roles (this ABI)

| Reg  | Role                                            | Saved by  |
| ---- | ----------------------------------------------- | --------- |
| ACC  | first scalar arg; scalar return value           | caller    |
| EXT  | high half of 32-bit return; ACC+EXT pair        | caller    |
| XR1  | scratch + parameter-list pointer in callee      | caller    |
| XR2  | frame base (locals + spills); doubles as SP     | callee    |
| XR3  | **LIBF transfer-vector base -- never modified** | loader    |
| IAR  | program counter                                 | hardware  |

"caller" = caller must save before the call if it needs the value
afterward. "callee" = callee must save on entry and restore on exit
if it modifies the register.

**XR3 is reserved.** This is non-negotiable: standard 1130 software
addresses every library subprogram (LIBF) through XR3, and any code
that wants to interoperate with FORTRAN, the Disk Monitor System
I/O subroutines, or any IBM-supplied library must leave XR3 alone for
the program's entire lifetime. The loader sets XR3 to the transfer-
vector base; user code (including ours) must never write it.

The general-purpose register class for the allocator is `{ACC, XR1}`.
EXT is reserved for individual allocation (only used as the low half
of the ACC+EXT pair); XR2 is reserved as the frame base; XR3 is
reserved for the LIBF base.

## 3. Argument passing

Following the standard CALL idiom (see Subroutine Library p. v and
ibm1130.net's "Programming Tips and Techniques"):

- The first scalar argument (16-bit or smaller) is passed in **ACC**.
- The first 32-bit argument is passed in the **ACC+EXT** pair (ACC =
  high word, EXT = low word).
- Subsequent arguments are passed **in-line after the BSI** as DC
  words. Each DC holds either an immediate value (for scalars whose
  address is the constant itself) or the address of the actual
  argument (for by-reference passing, FORTRAN style).
- Arguments larger than 32 bits (I64/U64, structs, arrays) are
  passed by reference: the caller emits a DC with the argument's
  address; the callee reads through it.
- Pointers are 1 word (16 bits) and pass like any other 16-bit
  scalar: ACC for the first slot, then in-line DC words.

The callee reads its in-line arguments by indexing through the return-
address slot. After the BSI, the entry word at NAME holds the return
address (which is also the address of the first DC). On return, the
callee bumps the return address past the parameter block; control
flows to the instruction after the last DC.

Variadic functions are **not supported** in this ABI. A future
extension could add an explicit args-area pointer; it is out of scope
for the bring-up.

LIBF linkage (one-word call through an XR3-relative transfer vector)
is **not emitted by our codegen** in the bring-up scope. LIBF would
require building a transfer-vector pass at link time. The CALL idiom
above is self-contained and is what step-9 codegen exercises.

## 4. Return value

- 16-bit scalars (I8, U8, I16, U16, Bool, Ptr): in **ACC**.
- 32-bit scalars (I32, U32): in the **ACC+EXT** pair (ACC = high, EXT
  = low). This is the natural shape for `M` and `D` results.
- 64-bit scalars (I64, U64): returned in memory at a caller-provided
  hidden first argument; the address goes in XR1 and the callee
  writes four words there.
- Aggregates and structs: same as I64 -- caller-provided hidden
  pointer in XR1; callee writes by indexing.

## 5. Caller-saved vs callee-saved

Caller-saved (volatile across calls): **ACC, EXT, XR1**.
Callee-saved (preserved across calls): **XR2**.
Reserved (program-lifetime invariant): **XR3** (LIBF base).
Hardware-managed: **IAR** (BSI writes it; branches modify it).

Rationale:

- ACC and EXT are the only arithmetic registers; making them callee-
  saved would force every leaf function to save/restore them, which
  is wasteful when most calls happen mid-expression. This matches
  the historical IBM convention -- all ISSs save and restore ACC/EXT
  internally precisely because callers cannot rely on them.
- XR1 is the codegen scratch and the conventional FORTRAN parameter
  pointer; making it caller-saved matches FORTRAN practice and lets
  callees use it freely.
- XR2 is callee-saved because it holds the activation record's frame
  base; it must survive a call.
- XR3 is the LIBF transfer-vector base -- preserved by the loader for
  the program's lifetime, never modified by user code.

## 6. Stack and frame base

The 1130 has no hardware stack pointer. **This ABI does not define a
separate stack pointer**: activation records are fixed-size at compile
time, and the same register (XR2) serves as both the frame base and
the logical stack pointer.

- **Frame base / stack pointer**: held in **XR2**. On routine entry,
  XR2 points to the current activation record's base. The callee may
  decrement XR2 to allocate locals (in functions large enough to need
  it) and restores XR2 on exit. For most functions, a fixed-size
  pre-allocated frame in static memory is enough and XR2 is not
  touched at all.
- **Growth direction**: **down** (toward lower addresses). This
  matches modern conventions and makes "decrement to allocate"
  natural; the 1130's historical FORTRAN runtime did not need any
  growth direction because frames were static.
- **Alignment**: **1 word** (1 address-unit). The 1130 cannot
  address sub-word values, so word alignment is the only alignment.
- **Recursion**: not in initial scope. The fixed-frame model rules
  out recursion; supporting it would require a real stack discipline
  (decrement XR2 on entry, restore on exit) and is left for a future
  step.

The trait `CallingConvention` requires both `stack_pointer` and
`frame_pointer`. We return XR2 for `stack_pointer` and `None` for
`frame_pointer`: there is no distinct FP; XR2 is the single frame
base. This keeps the trait honest -- codegen cannot accidentally use
a different register for SP vs FP because the ABI says they
coincide.

## 7. Frame layout

Each activation record is laid out at fixed offsets relative to XR2.
For a function with no nested calls, the frame may live entirely in
static memory addressable by the assembler-supplied symbol; only
recursion-capable or large-frame functions actually adjust XR2.

```
high addresses
+-------------------------+
| caller-supplied         |  inline DC parameters following the
| in-line parameters      |  caller's BSI; addressed via the
|                         |  return-address slot at NAME
+-------------------------+
| local var 1             |
| local var 2             |
| ...                     |
+-------------------------+
| spill slot 1            |
| spill slot 2            |
| ...                     |
+-------------------------+
| outgoing-arg scratch    |  for nested calls; addresses written
|                         |  before BSI, read by the callee
+-------------------------+   <- XR2 (frame base / SP)
low addresses
```

Frame slots are word-addressed; the prologue computes the frame size
in words at compile time and (for stack-using functions) decrements
XR2 by that much.

## 8. Prologue / epilogue sketch

Most generated functions need no prologue or epilogue: the activation
record is a static memory block, parameters arrive through in-line
DCs, and the function body operates directly on those slots.

For functions large enough to need a stack-style frame (or, in a
future revision, recursion):

Prologue:

```
    STX  2 SAVE_XR2          ; save caller's frame base
    SUB  XR2, frame_size     ; allocate frame (descending stack)
```

Epilogue:

```
    LDX  2 SAVE_XR2          ; restore caller's frame base
    BSC  I NAME              ; return: indirect through entry word
```

The return is the standard 1130 idiom: `BSC I NAME` reads the return-
address word that BSI wrote into NAME, adjusts it past the parameter
block (callee-known constant), and jumps. Codegen knows the parameter
count from the call signature.

## 9. System call interface

Deferred. A real 1130 system call would be a BSI to a stub at a
well-known address belonging to the System Director (the 1130's
operating system) or a LIBF call into a Disk Monitor I/O subroutine.
Real BSI-to-SIB (System Indicator Block) and LIBF emission are **out
of scope** for this bring-up; codegen treats system calls as opaque
external symbols resolved by the linker / loader.

## 10. Type widths and alignment

All sizes are in **address-units** (1 word = 16 bits = 2 bytes).
Pointer width is 16 bits.

| Type | Width (words) | Alignment (words) | Storage notes              |
| ---- | ------------- | ----------------- | -------------------------- |
| I8   | 1             | 1                 | sign-extended into 16 bits |
| U8   | 1             | 1                 | zero-extended into 16 bits |
| I16  | 1             | 1                 | natural word               |
| U16  | 1             | 1                 | natural word               |
| I32  | 2             | 1                 | high word, then low word   |
| U32  | 2             | 1                 | high word, then low word   |
| I64  | 4             | 1                 | manual lowering by codegen |
| U64  | 4             | 1                 | manual lowering by codegen |
| Bool | 1             | 1                 | 0 = false, nonzero = true  |
| Ptr  | 1             | 1                 | 16-bit word address        |

The 1130 has no sub-word access; bytes pad to a full word. Codegen is
responsible for masking on signed/unsigned narrowing.

## 11. Open questions for later steps

These are deliberately deferred and listed here for the postmortem
step (saga step 12) to revisit:

- **LIBF emission.** If the toolchain ever needs to call IBM library
  subprograms or interoperate with FORTRAN object code, codegen
  needs to emit LIBF call sequences and a linker pass needs to build
  the XR3-based transfer vector. Out of scope for the bring-up.
- **Recursion / dynamic frames.** The fixed-frame model precludes
  recursion. Adding it would mean decrementing XR2 on entry and
  restoring on exit; the trait surface already permits it, but
  codegen does not yet emit those sequences.
- **`M` / `D` partial-pair use.** Multiply produces ACC+EXT; if
  codegen ever wants ACC alone after a multiply that needed the
  high half, the fixed-pair allocator may need to spill EXT
  explicitly. Has not yet bitten in practice.
- **Initial XR2 value.** The frame-base register must be initialised
  by program startup; the exact memory location is a memory-map
  question (TBD) that the emulator step (step 11) will pin down.
