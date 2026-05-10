# IBM 1130 ABI

Status: invented for this toolchain. The IBM 1130 manuals do not specify
a calling convention -- routine entry/exit was per-program in the era's
assembly idiom, and the FORTRAN/CALL subroutine library used its own
conventions tied to IBM's runtime. This document defines the ABI that
`sw-ibm1130-target`, `sw-ibm1130-codegen`, and downstream code agree
on.

References:

- `gen-isa/docs/decisions.md` Sec 1 (XR3 reserved as frame pointer).
- `gen-isa/docs/porting-guide.md` Sec 5 (ABI invention guidance).
- IBM 1130 Functional Characteristics (GA26-5881) for hardware register
  state and instruction semantics.

ASCII-only by convention.

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
register. Subroutine calls use BSI (Branch and Store IAR), which writes
the return address into the word at the call target and jumps to
target+1; this makes the simplest recursion-free call shape natural,
and forces software to manage any stack discipline.

Memory is word-addressed: 1 address-unit = 1 16-bit word = 2 bytes.
Word alignment is the only alignment that exists.

## 2. Register roles (this ABI)

| Reg  | Role                                       | Saved by  |
| ---- | ------------------------------------------ | --------- |
| ACC  | first scalar arg; scalar return value      | caller    |
| EXT  | high half of 32-bit return; ACC+EXT pair   | caller    |
| XR1  | scratch index / second arg slot pointer    | caller    |
| XR2  | logical stack pointer                      | callee    |
| XR3  | frame pointer                              | callee    |
| IAR  | program counter                            | hardware  |

"caller" = caller must save before the call if it needs the value
afterward. "callee" = callee must save on entry and restore on exit if
it modifies the register.

The general-purpose register class for the allocator is `{ACC, XR1}`.
EXT is reserved for individual allocation (only used as the low half
of the ACC+EXT pair); XR2 and XR3 are reserved for SP and FP.

## 3. Argument passing

- The first scalar argument (16-bit or smaller) is passed in **ACC**.
- The first 32-bit argument is passed in the **ACC+EXT** pair (ACC =
  high word, EXT = low word).
- Subsequent arguments are passed in memory at fixed slots adjacent to
  the call site. Concretely: the caller emits the additional arg words
  immediately after the BSI long-form word, and the callee reads them
  by indexing through XR1 set to the return address minus one.
- Arguments larger than 32 bits (I64/U64) are passed entirely in
  memory, by value, in the same call-site arg block.
- Pointers are 1 word (16 bits) and pass like any other 16-bit scalar:
  ACC for the first slot, then the in-memory arg block.

The "args after the BSI" arrangement matches the historical 1130
subroutine library shape (the standard "load XR1 from IAR, then read
arg cells via STX/LDX off XR1") and avoids needing dynamic stack
discipline for non-recursive code.

Variadic functions are **not supported** in this ABI. A future
extension could add an explicit args-area pointer; it is out of scope
for the bring-up.

## 4. Return value

- 16-bit scalars (I8, U8, I16, U16, Bool, Ptr): in **ACC**.
- 32-bit scalars (I32, U32): in the **ACC+EXT** pair (ACC = high, EXT
  = low). This is the natural shape for `M` and `D` results.
- 64-bit scalars (I64, U64): returned in memory at a caller-provided
  hidden first argument; the address goes in XR1 and the callee writes
  four words there.
- Aggregates and structs: same as I64 -- caller-provided hidden
  pointer in XR1; callee writes by indexing.

## 5. Caller-saved vs callee-saved

Caller-saved (volatile across calls): **ACC, EXT, XR1**.
Callee-saved (preserved across calls): **XR2, XR3**.
IAR is hardware-managed (BSI writes it, branches modify it).

Rationale:

- ACC and EXT are the only arithmetic registers; making them
  callee-saved would force every leaf function to save/restore them,
  which is wasteful when most calls happen mid-expression.
- XR1 is the codegen scratch pointer -- caller-saved so callees can
  trash it freely.
- XR2 (SP) and XR3 (FP) hold the activation record state and must
  survive a call.

## 6. Stack

The 1130 has no hardware stack pointer. This ABI defines a **logical
stack**:

- **Stack pointer**: held in **XR2**. On routine entry, XR2 points to
  the current activation record's top (lowest address used). The
  callee may decrement XR2 (subtract from the index register) to
  allocate locals, and restores XR2 on exit.
- **Growth direction**: **down** (toward lower addresses), matching
  most modern conventions and making "decrement to allocate" natural.
- **Alignment**: **1 word** (1 address-unit). The 1130 cannot address
  sub-word values, so word alignment is the only alignment.
- **Initial SP**: program startup loads XR2 with a word in low memory
  (TBD: codegen + emulator agree on a fixed location, e.g. word
  address 0x4000) and grows downward from there.

For non-recursive code (the common 1130 case), the SP discipline is
optional -- a routine may use fixed memory cells for locals without
touching XR2 at all. The codegen will emit the SP-touching prologue
only when the function is recursive or has large stack-allocated
objects.

## 7. Frame layout

Stack grows toward lower addresses. The frame pointer (XR3) is set on
entry and points just below the saved-XR3 slot, so locals and spills
are addressed via positive offsets and incoming args via negative
offsets relative to FP.

```
high addresses
+------------------+
| caller args N..  |  passed in memory by caller (above FP)
+------------------+
| caller args 1..N |
+------------------+   <- FP+1 (caller frame top)
| saved FP (XR3)   |
+------------------+   <- FP (XR3 = address of this slot's address)
| saved XR2 (SP)   |   (only if the callee modifies SP)
+------------------+
| saved callee     |   (other callee-saved values, currently none)
| state            |
+------------------+
| local var 1      |
| local var 2      |
| ...              |
+------------------+
| spill slot 1     |
| spill slot 2     |
| ...              |
+------------------+
| outgoing args    |   (for nested calls; written before BSI)
+------------------+   <- SP (XR2)
low addresses
```

Frame slots are word-addressed; the prologue computes the frame size
in words at compile time and decrements XR2 by that much.

## 8. Prologue / epilogue sketch

Prologue (recursive or stack-using function):

```
    STX  XR3, FP_save_slot       ; save caller's FP
    LDX  XR3, XR2                ; FP <- current SP
    SUB  XR2, frame_size         ; allocate frame
    ; (no callee-saves besides FP for current spec)
```

Epilogue:

```
    LDX  XR2, XR3                ; SP <- FP
    LDX  XR3, FP_save_slot       ; restore caller's FP
    BSC  ...                     ; return (BSC with appropriate condition)
```

For non-recursive leaf functions, both prologue and epilogue collapse
to nothing; the function body uses fixed memory cells for locals.

## 9. System call interface

Deferred. A real 1130 system call would be a BSI to a stub at a
well-known address belonging to the System Director (the 1130's
operating system). Real BSI-to-SIB (System Indicator Block) is **out
of scope** for this bring-up; codegen treats system calls as opaque
external symbols resolved by the linker / loader.

## 10. Type widths and alignment

All sizes are in **address-units** (1 word = 16 bits). Pointer width
is 16 bits.

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

- The "args after BSI" arg-passing scheme conflates arg layout with
  call-site code emission; it may force codegen to know the arg list
  during BSI emission. If this becomes painful, a stack-based arg
  ABI is a fallback.
- `M` and `D` produce ACC+EXT results; if the codegen ever wants to
  use ACC alone after a multiply that the higher half is needed for,
  the fixed-pair allocator may need to spill EXT explicitly.
- The exact initial SP value (a memory-map question, not strictly
  ABI) is TBD until the emulator step (step 11).
