#ifndef KERNEL_RAM_PAGE
KERNEL_RAM_PAGE             .equ 3
#endif
#if KERNEL_RAM_PAGE > 7
 .error "KERNEL_RAM_PAGE must be a RAM page, 0 to 7"
#endif
#ifndef KERNEL_RAM_LOW
KERNEL_RAM_LOW              .equ $C500
#endif
#ifndef KERNEL_RAM_HIGH
KERNEL_RAM_HIGH             .equ $DF00
#endif
LOW_SIZE                    .equ $1200
HIGH_SIZE                   .equ $2000
#if KERNEL_RAM_LOW & $FF
 .error "KERNEL_RAM_LOW must be a multiple of $100"
#endif
#if KERNEL_RAM_HIGH & $FF
 .error "KERNEL_RAM_HIGH must be a multiple of $100"
#endif
#if KERNEL_RAM_LOW < $C000
 .error "KERNEL_RAM_LOW must be in bank C"
#endif
#if KERNEL_RAM_HIGH < $C000
 .error "KERNEL_RAM_HIGH must be in bank C"
#endif
KERNEL_RAM_LOW_END          .equ KERNEL_RAM_LOW+LOW_SIZE
KERNEL_RAM_HIGH_END         .equ KERNEL_RAM_HIGH+HIGH_SIZE
#if KERNEL_RAM_LOW_END > $10000
 .error "the low region does not fit below $10000"
#endif
#if KERNEL_RAM_HIGH_END > $10000
 .error "the high region does not fit below $10000"
#endif
#if KERNEL_RAM_LOW < KERNEL_RAM_HIGH_END
#if KERNEL_RAM_HIGH < KERNEL_RAM_LOW_END
 .error "the low and high regions overlap"
#endif
#endif
KERNEL_LIB_PAGE             .equ KERNEL_RAM_LOW+$00
KERNEL_LIMIT_STACK_SYMS     .equ KERNEL_RAM_LOW+$01
KERNEL_LIMIT_STACK_ENTRIES  .equ KERNEL_RAM_LOW+$03
KERNEL_LIMIT_SAVED_SYMS     .equ KERNEL_RAM_LOW+$05
KERNEL_LIMIT_SAVED_ENTRIES  .equ KERNEL_RAM_LOW+$07
PARAM_LEN                   .equ 9

kv_saved_sp         .equ KERNEL_RAM_LOW+$10
kv_saved_bank       .equ KERNEL_RAM_LOW+$12
kv_saved_iff        .equ KERNEL_RAM_LOW+$13
kv_saved_port5      .equ KERNEL_RAM_LOW+$14
kv_ready            .equ KERNEL_RAM_LOW+$A0
kv_work             .equ KERNEL_RAM_LOW+$B0
kv_wold             .equ KERNEL_RAM_LOW+$B3
kv_charge           .equ KERNEL_RAM_LOW+$B4
kv_next             .equ KERNEL_RAM_LOW+$B6
kv_crc_state        .equ KERNEL_RAM_LOW+$A2
kv_crc_chunk        .equ KERNEL_RAM_LOW+$A4
kv_crc_last         .equ KERNEL_RAM_LOW+$A5
kv_crc_end          .equ KERNEL_RAM_LOW+$A6
KERNEL_CRC          .equ KERNEL_RAM_LOW+$A8
kv_crc_stop         .equ KERNEL_RAM_LOW+$AC
kv_crc_rem          .equ KERNEL_RAM_LOW+$AE
kv_crc_bits         .equ KERNEL_RAM_LOW+$AF
READY_MAGIC         .equ $5AA5
kv_arg_id           .equ KERNEL_RAM_LOW+$15
kv_constants        .equ KERNEL_RAM_LOW+$17
kv_symbols          .equ KERNEL_RAM_LOW+$18
kv_assertions       .equ KERNEL_RAM_LOW+$19
kv_directory        .equ KERNEL_RAM_LOW+$1B
kv_length           .equ KERNEL_RAM_LOW+$1E
kv_len_chunk        .equ KERNEL_RAM_LOW+$21
kv_len_tail         .equ KERNEL_RAM_LOW+$22
kv_rd_off           .equ KERNEL_RAM_LOW+$24
kv_sym_id           .equ KERNEL_RAM_LOW+$27
kv_tmp8             .equ KERNEL_RAM_LOW+$28
kv_mode             .equ KERNEL_RAM_LOW+$29
kv_tmp24            .equ KERNEL_RAM_LOW+$2A
kv_tmp16            .equ KERNEL_RAM_LOW+$2D
kv_rec_end          .equ KERNEL_RAM_LOW+$2F
kv_sym_limit_ptr    .equ KERNEL_RAM_LOW+$31
kv_ent_limit        .equ KERNEL_RAM_LOW+$33
kv_saved_sym_ptr    .equ KERNEL_RAM_LOW+$35
kv_saved_ent_limit  .equ KERNEL_RAM_LOW+$37
kv_stack_top        .equ KERNEL_RAM_LOW+$39
kv_nstack           .equ KERNEL_RAM_LOW+$3B
kv_saved_top        .equ KERNEL_RAM_LOW+$3D
kv_nsaved           .equ KERNEL_RAM_LOW+$3F
kv_proof_ptr        .equ KERNEL_RAM_LOW+$41
kv_proof_end        .equ KERNEL_RAM_LOW+$43
kv_pair             .equ KERNEL_RAM_LOW+$45
kv_serial           .equ KERNEL_RAM_LOW+$47
kv_base             .equ KERNEL_RAM_LOW+$49
kv_entry0           .equ KERNEL_RAM_LOW+$4B
kv_starts_end       .equ KERNEL_RAM_LOW+$4D
kv_var              .equ KERNEL_RAM_LOW+$4F
kv_wr               .equ KERNEL_RAM_LOW+$50
kv_xs               .equ KERNEL_RAM_LOW+$52
kv_xe               .equ KERNEL_RAM_LOW+$54
kv_ys               .equ KERNEL_RAM_LOW+$56
kv_ye               .equ KERNEL_RAM_LOW+$58
kv_xp               .equ KERNEL_RAM_LOW+$5A
kv_yp               .equ KERNEL_RAM_LOW+$5C
kv_a                .equ KERNEL_RAM_LOW+$5E
kv_lib_page         .equ KERNEL_RAM_LOW+$5F
CUR_DESC            .equ KERNEL_RAM_LOW+$60
USED_DESC           .equ KERNEL_RAM_LOW+$80

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

TYPECODES           .equ KERNEL_RAM_LOW+$100
FLOATS              .equ KERNEL_RAM_LOW+$200
KSTACK              .equ KERNEL_RAM_LOW+$300
KSTACK_TOP          .equ KERNEL_RAM_LOW+$400
CACHE               .equ KERNEL_RAM_LOW+$400
STARTS_BUF          .equ KERNEL_RAM_LOW+$800
SAVED_STARTS_BUF    .equ KERNEL_RAM_LOW+$A00
CRC_TABLE           .equ KERNEL_RAM_LOW+$E00
SUBST               .equ KERNEL_RAM_HIGH+$000
STACK_BUF           .equ KERNEL_RAM_HIGH+$800
SAVED_BUF           .equ KERNEL_RAM_HIGH+$1400

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
ST_WORK_LIMIT       .equ 13
ST_INTERRUPTED      .equ 14
ST_LOOSE_REF        .equ 15
ST_CRC_MORE         .equ $FF

WORK_LIMIT          .equ 131072
#if WORK_LIMIT < 1 || WORK_LIMIT > $FFFFFF
 .error "WORK_LIMIT must fit the 24-bit work counter"
#endif
