#include "fm/ram.inc"

; The walk lies in the bytes of the spare page that fm's statement store and FM_LOW give up. In
; FM_TOKENS, 3 bytes in 4 of the walk's stack and tables, all of its $C00:
PW_ENTRIES          .equ FM_TOKENS
PW_ENTRIES_SIZE     .equ $100
PW_STARTS           .equ FM_TOKENS + $100
PW_STARTS_SIZE      .equ $200
PW_SAVED            .equ FM_TOKENS + $300
PW_SAVED_SIZE       .equ $200
PW_SIZES            .equ FM_TOKENS + $500
PW_SIZES_SIZE       .equ $300
PW_MEMO             .equ FM_TOKENS + $800
PW_MAX_REFS         .equ 512
PW_MEMO_SIZE        .equ PW_MAX_REFS * 2
PW_TOKENS_USED      .equ PW_MEMO + PW_MEMO_SIZE - FM_TOKENS
#if PW_TOKENS_USED != FM_TOKENS_SIZE
 .error "the walk's tables are meant to fill fm's token store"
#endif
#if PW_SIZES_SIZE < 3 * 255
 .error "a size is 3 bytes and a proof has at most 255 goals"
#endif

; In FM_LOW from +$100 for $D00 bytes: the variables, the goal table with one column for each
; field, two bytes a goal in the first four, the hypothesis ranks, the subgoal links, and the stack
; at the top.
PW_LOW              .equ FM_LOW + $100
PW_LOW_SIZE         .equ $D00
#if PW_LOW & $FF
 .error "PW_LOW must be a multiple of $100: the sum of the variables ends at a page's start"
#endif
PW_VARS             .equ PW_LOW
PW_VARS_SIZE        .equ $80
PW_MAX_GOALS        .equ 255
PW_COLUMN           .equ 2 * PW_MAX_GOALS
PW_STEP             .equ PW_VARS + PW_VARS_SIZE
PW_BEGIN            .equ PW_STEP + PW_COLUMN
PW_BY               .equ PW_BEGIN + PW_COLUMN
PW_LSTART           .equ PW_BY + PW_COLUMN
PW_HYPK             .equ PW_LSTART + PW_COLUMN
PW_HYPK_SIZE        .equ 255
PW_LINKS            .equ PW_HYPK + PW_HYPK_SIZE
PW_MAX_LINKS        .equ 857
PW_STACK_TOP        .equ PW_LOW + PW_LOW_SIZE
PW_STACK_ROOM       .equ $10
#if (PW_LINKS + PW_MAX_LINKS) > (PW_STACK_TOP - PW_STACK_ROOM)
 .error "the links table runs into the stack's room"
#endif
PW_MAX_SUBGOALS     .equ 31
#if PW_MAX_LINKS < 3 * 255
 .error "the links table must hold at least 765 links"
#endif
#if (PW_LOW + PW_LOW_SIZE) > (FM_LOW + $F00)
 .error "the walk may use FM_LOW only to +$E00"
#endif

#define PW_NEXT 0
#macro PW_VAR(name, offset, size)
RAM_VAR(name, PW_VARS, offset, size, PW_VARS_SIZE, PW_NEXT, "the walk's variables overlap, leave a gap, or run past PW_VARS_SIZE")
#define PW_NEXT eval(offset + size)
#endmacro

; The mark, then the variables a walk keeps from one call to the next, then their sum. Only a
; mark that matches the sum lets a continue or a find act on what it finds.
PW_VAR(pv_mark, $00, 1)
PW_VAR(pv_stage, $01, 1)
PW_VAR(pv_tc, $02, 1)
PW_VAR(pv_lpage, $03, 1)
PW_VAR(pv_rpage, $04, 1)
PW_VAR(pv_dir, $05, 3)
PW_VAR(pv_len, $08, 3)
PW_VAR(pv_id, $0B, 2)
PW_VAR(pv_nhyps, $0D, 1)
PW_VAR(pv_nrefs, $0E, 2)
PW_VAR(pv_hyps, $10, 2)
PW_VAR(pv_refs, $12, 2)
PW_VAR(pv_proof, $14, 2)
PW_VAR(pv_pend, $16, 2)
PW_VAR(pv_pp, $18, 2)
PW_VAR(pv_hrem, $1A, 1)
PW_VAR(pv_hk, $1B, 2)
PW_VAR(pv_rank, $1D, 1)
PW_VAR(pv_ri, $1E, 2)
PW_VAR(pv_rp, $20, 2)
PW_VAR(pv_mp, $22, 2)
PW_VAR(pv_steps, $24, 2)
PW_VAR(pv_goals, $26, 1)
PW_VAR(pv_links, $27, 2)
PW_VAR(pv_total, $29, 3)
PW_VAR(pv_nstack, $2C, 2)
PW_VAR(pv_nsaved, $2E, 2)
PW_VAR(pv_sum, $30, 2)
PW_PERSIST          .equ pv_stage
PW_PERSIST_END      .equ pv_sum
#if (PW_PERSIST_END - PW_PERSIST) > $FF
 .error "the low byte of the end of the persistent variables must name it"
#endif
PW_RESULT           .equ pv_steps
PW_RESULT_SIZE      .equ 8
PW_CLEAR_SIZE       .equ PW_PERSIST_END - pv_steps

; Variables of one call.
PW_VAR(pv_sp, $32, 2)
PW_VAR(pv_p5, $34, 1)
PW_VAR(pv_p7, $35, 1)
PW_VAR(pv_iff, $36, 1)
PW_VAR(pv_mode, $37, 1)
PW_VAR(pv_cut, $38, 1)
PW_VAR(pv_left, $39, 2)
PW_VAR(pv_blk, $3B, 2)
PW_VAR(pv_status, $3D, 1)
PW_VAR(pv_cnt, $3E, 1)
PW_VAR(pv_t1, $3F, 2)
PW_VAR(pv_t2, $41, 2)
PW_VAR(pv_acc, $43, 3)
PW_VAR(pv_step, $46, 2)
PW_VAR(pv_begin, $48, 2)
PW_VAR(pv_by, $4A, 2)
PW_VAR(pv_ls, $4C, 2)
PW_VAR(pv_nc, $4E, 1)
PW_VAR(pv_nh, $4F, 1)
PW_VAR(pv_lim, $50, 2)
PW_VAR(pf_cur, $52, 1)
PW_VAR(pf_depth, $53, 1)
PW_VAR(pf_idx, $54, 1)
PW_VAR(pf_c, $55, 1)
PW_VAR(pf_n, $56, 1)
PW_VAR(pf_g, $57, 1)
PW_VAR(pf_ls, $58, 2)
PW_VAR(pf_place, $5A, 3)
PW_VAR(pf_cp, $5D, 3)
PW_VAR(pf_cs, $60, 3)
PW_VAR(pf_ce, $63, 3)
PW_VAR(pf_parent, $66, 3)
PW_USED             .equ $69

; What a find writes in the caller's block.
PW_FIND_SIZE        .equ 16 + PW_MAX_SUBGOALS

; The entry goes in a stack entry's byte for a syntax statement.
PW_SYNTAX           .equ $FF
PW_MARK_RUN         .equ $5A
PW_MARK_DONE        .equ $A5

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
PW_ST_MORE             .equ 45
PW_ERR_NO_WALK      .equ 46
PW_ERR_PLACE        .equ 47
PW_ERR_NO_TABLE     .equ 48
PW_ERR_LINKS        .equ 49
PW_ERR_SUBGOALS     .equ 50
