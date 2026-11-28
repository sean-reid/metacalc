#include "fm/ram.inc"

#if FM_TOKENS & $FF
 .error "FM_TOKENS must be a multiple of $100: the walk's stack indexes it by one byte"
#endif

; The walk's tables, on the spare page in fm's statement store: the stack of entries, the saved
; entries, three bytes of subtree size for each goal, a byte for each hypothesis of the theorem and
; two bytes for each reference.
PW_ENTRIES          .equ FM_TOKENS
PW_ENTRIES_SIZE     .equ $100
PW_SAVED            .equ FM_TOKENS + $100
PW_SAVED_SIZE       .equ $200
PW_SIZES            .equ FM_TOKENS + $300
PW_SIZES_SIZE       .equ $300
PW_HYPK             .equ FM_TOKENS + $600
PW_HYPK_SIZE        .equ $100
PW_MEMO             .equ FM_TOKENS + $700
PW_MAX_REFS         .equ 512
PW_MEMO_SIZE        .equ PW_MAX_REFS * 2
PW_TOKENS_USED      .equ PW_MEMO + PW_MEMO_SIZE - FM_TOKENS
#if PW_TOKENS_USED > FM_TOKENS_SIZE
 .error "the walk's tables are larger than fm's token store"
#endif
#if PW_SIZES_SIZE < 3 * 255
 .error "a size is 3 bytes and a proof has at most 255 goals"
#endif

; The variables and the stack, at the start of fm's private stack page: the stack grows down from
; FM_STACK_TOP and never reaches them.
PW_VARS             .equ FM_STACK
PW_VARS_SIZE        .equ $40
PW_STACK_TOP        .equ FM_STACK_TOP
#define PW_NEXT 0
#macro PW_VAR(name, offset, size)
RAM_VAR(name, PW_VARS, offset, size, PW_VARS_SIZE, PW_NEXT, "the walk's variables overlap, leave a gap, or run past PW_VARS_SIZE")
#define PW_NEXT eval(offset + size)
#endmacro

PW_VAR(pv_sp, $00, 2)
PW_VAR(pv_p5, $02, 1)
PW_VAR(pv_p7, $03, 1)
PW_VAR(pv_iff, $04, 1)
PW_VAR(pv_tc, $05, 1)
PW_VAR(pv_lpage, $06, 1)
PW_VAR(pv_rpage, $07, 1)
PW_VAR(pv_dir, $08, 3)
PW_VAR(pv_len, $0B, 3)
PW_VAR(pv_id, $0E, 2)
PW_VAR(pv_nhyps, $10, 1)
PW_VAR(pv_nrefs, $11, 2)
PW_VAR(pv_hyps, $13, 2)
PW_VAR(pv_refs, $15, 2)
PW_VAR(pv_proof, $17, 2)
PW_VAR(pv_pend, $19, 2)
PW_VAR(pv_pp, $1B, 2)
PW_VAR(pv_lim, $1D, 2)
PW_VAR(pv_cnt, $1F, 1)
PW_VAR(pv_nh, $20, 1)
PW_VAR(pv_ri, $21, 2)
PW_VAR(pv_rp, $23, 2)
PW_VAR(pv_mp, $25, 2)
PW_VAR(pv_t1, $27, 2)
PW_VAR(pv_t2, $29, 2)
PW_VAR(pv_acc, $2B, 3)
PW_VAR(pv_status, $2E, 1)
PW_VAR(pv_steps, $2F, 2)
PW_VAR(pv_goals, $31, 1)
PW_VAR(pv_links, $32, 2)
PW_VAR(pv_total, $34, 3)
PW_VAR(pv_nstack, $37, 2)
PW_VAR(pv_nsaved, $39, 2)
PW_USED             .equ $3B
PW_RESULT           .equ pv_steps
PW_RESULT_SIZE      .equ 8
PW_CLEAR_SIZE       .equ PW_USED - $2F

PW_ERR_UNDERFLOW    .equ 32
PW_ERR_BAD_REF      .equ 33
PW_ERR_FORWARD      .equ 34
PW_ERR_SAVE_EMPTY   .equ 35
PW_ERR_NO_ENTRY     .equ 36
PW_ERR_MANY_ENTRIES .equ 37
PW_ERR_GOALS        .equ 38
PW_ERR_STACK        .equ 39
PW_ERR_SAVED        .equ 40
PW_ERR_CUT          .equ 41
PW_ERR_SYNTAX       .equ 42
PW_ERR_TOTAL        .equ 43
PW_ERR_REFS         .equ 44
