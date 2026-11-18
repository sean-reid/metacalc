#ifndef KERNEL_RAM
KERNEL_RAM                  .equ $C000
#endif
#if KERNEL_RAM & $FF
 .error "KERNEL_RAM must be a multiple of $100"
#endif
#if KERNEL_RAM < $C000
 .error "KERNEL_RAM must be in bank C"
#endif
KERNEL_LIB_PAGE             .equ KERNEL_RAM+$00
KERNEL_LIMIT_STACK_SYMS     .equ KERNEL_RAM+$01
KERNEL_LIMIT_STACK_ENTRIES  .equ KERNEL_RAM+$03
KERNEL_LIMIT_SAVED_SYMS     .equ KERNEL_RAM+$05
KERNEL_LIMIT_SAVED_ENTRIES  .equ KERNEL_RAM+$07

kv_saved_sp         .equ KERNEL_RAM+$10
kv_saved_bank       .equ KERNEL_RAM+$12
kv_saved_iff        .equ KERNEL_RAM+$13
kv_ready            .equ KERNEL_RAM+$A0
READY_MAGIC         .equ $5AA5
kv_arg_id           .equ KERNEL_RAM+$15
kv_constants        .equ KERNEL_RAM+$17
kv_symbols          .equ KERNEL_RAM+$18
kv_assertions       .equ KERNEL_RAM+$19
kv_directory        .equ KERNEL_RAM+$1B
kv_length           .equ KERNEL_RAM+$1E
kv_len_chunk        .equ KERNEL_RAM+$21
kv_len_tail         .equ KERNEL_RAM+$22
kv_rd_off           .equ KERNEL_RAM+$24
kv_sym_id           .equ KERNEL_RAM+$27
kv_tmp8             .equ KERNEL_RAM+$28
kv_mode             .equ KERNEL_RAM+$29
kv_tmp24            .equ KERNEL_RAM+$2A
kv_tmp16            .equ KERNEL_RAM+$2D
kv_rec_end          .equ KERNEL_RAM+$2F
kv_sym_limit_ptr    .equ KERNEL_RAM+$31
kv_ent_limit        .equ KERNEL_RAM+$33
kv_saved_sym_ptr    .equ KERNEL_RAM+$35
kv_saved_ent_limit  .equ KERNEL_RAM+$37
kv_stack_top        .equ KERNEL_RAM+$39
kv_nstack           .equ KERNEL_RAM+$3B
kv_saved_top        .equ KERNEL_RAM+$3D
kv_nsaved           .equ KERNEL_RAM+$3F
kv_proof_ptr        .equ KERNEL_RAM+$41
kv_proof_end        .equ KERNEL_RAM+$43
kv_pair             .equ KERNEL_RAM+$45
kv_serial           .equ KERNEL_RAM+$47
kv_base             .equ KERNEL_RAM+$49
kv_entry0           .equ KERNEL_RAM+$4B
kv_starts_end       .equ KERNEL_RAM+$4D
kv_var              .equ KERNEL_RAM+$4F
kv_wr               .equ KERNEL_RAM+$50
kv_xs               .equ KERNEL_RAM+$52
kv_xe               .equ KERNEL_RAM+$54
kv_ys               .equ KERNEL_RAM+$56
kv_ye               .equ KERNEL_RAM+$58
kv_xp               .equ KERNEL_RAM+$5A
kv_yp               .equ KERNEL_RAM+$5C
kv_a                .equ KERNEL_RAM+$5E
kv_lib_page         .equ KERNEL_RAM+$5F
CUR_DESC            .equ KERNEL_RAM+$60
USED_DESC           .equ KERNEL_RAM+$80

D_PAGE      .equ 0
D_KIND      .equ 1
D_NHYPS     .equ 2
D_HYPS      .equ 3
D_NDV       .equ 5
D_DV        .equ 7
D_NCONC     .equ 9
D_CONC      .equ 10
D_NEXTRA    .equ 12
D_EXTRA     .equ 14
D_NREFS     .equ 16
D_REFS      .equ 18
D_PLEN      .equ 20
D_PROOF     .equ 22
D_ID        .equ 24

TYPECODES           .equ KERNEL_RAM+$100
FLOATS              .equ KERNEL_RAM+$200
KSTACK              .equ KERNEL_RAM+$300
KSTACK_TOP          .equ KERNEL_RAM+$400
CACHE               .equ KERNEL_RAM+$400
SUBST               .equ KERNEL_RAM+$800
STARTS_BUF          .equ KERNEL_RAM+$1000
SAVED_STARTS_BUF    .equ KERNEL_RAM+$1200
STACK_BUF           .equ KERNEL_RAM+$1600
SAVED_BUF           .equ KERNEL_RAM+$2200
KERNEL_RAM_END      .equ KERNEL_RAM+$2E00
#if KERNEL_RAM_END > $10000
 .error "the kernel RAM block does not fit below $10000"
#endif

CACHE_IDS           .equ $2000
CAP_STACK_SYMS      .equ $0C00
CAP_STACK_ENTRIES   .equ 256
CAP_SAVED_SYMS      .equ $0C00
CAP_SAVED_ENTRIES   .equ 512

ST_OK               .equ 0
ST_MALFORMED        .equ 1
ST_BAD_RECORD       .equ 2
ST_NOT_A_THEOREM    .equ 3
ST_FORWARD_REF      .equ 4
ST_OUT_OF_MEMORY    .equ 5
ST_STACK_UNDERFLOW  .equ 6
ST_BAD_BACK_REF     .equ 7
ST_TYPECODE         .equ 8
ST_HYP_MISMATCH     .equ 9
ST_DISJOINT         .equ 10
ST_NOT_SINGLETON    .equ 11
ST_WRONG_CONCLUSION .equ 12
