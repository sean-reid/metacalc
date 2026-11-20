#include "kernel/ram.inc"
#if FM_RAM & $FF
 .error "FM_RAM must be a multiple of $100"
#endif

#ifndef FM_VARS_SIZE
FM_VARS_SIZE        .equ $85
#endif
#if FM_VARS_SIZE > $100
 .error "FM_VARS_SIZE must leave the stack at FM_RAM + $100 alone"
#endif
FM_STACK            .equ FM_RAM + $100
FM_STACK_TOP        .equ FM_RAM + $200
FM_GLYPHS           .equ FM_RAM + $200
FM_GLYPHS_SIZE      .equ $800
FM_SECOND           .equ FM_GLYPHS + FM_GLYPHS_SIZE
FM_SECOND_SIZE      .equ $800
FM_TOKENS           .equ FM_SECOND + FM_SECOND_SIZE
FM_MAX_TOKENS       .equ 768
FM_MAX_STMTS        .equ 32
FM_MAX_NODES        .equ 384
#ifndef FM_RUN_MAX
FM_RUN_MAX          .equ 128
#endif
#if (FM_RUN_MAX < 1) || (FM_RUN_MAX > 128)
 .error "FM_RUN_MAX must be 1 to 128: a run of tokens in FM_SCRATCH is cut at that many codes, a token has at most 128, and the terminator must fit"
#endif
FM_MAX_STMT_TOKENS  .equ 255
FM_MAX_STMT_NODES   .equ 255
FM_TOKENS_SIZE      .equ FM_MAX_TOKENS * 4
FM_RAM_USED         .equ FM_TOKENS + FM_TOKENS_SIZE - FM_RAM
#if FM_RAM_USED > FM_RAM_SIZE
 .error "FM_RAM_SIZE in ram_map.inc is smaller than what fm lays out"
#endif

#if FM_LOW & $FF
 .error "FM_LOW must be a multiple of $100"
#endif
FM_HDR              .equ FM_LOW
FM_HDR_SIZE         .equ FM_MAX_STMTS * 8
FM_TREE_SIZE        .equ $100
FM_TLEN             .equ FM_HDR + FM_HDR_SIZE
FM_TCNT             .equ FM_TLEN + FM_TREE_SIZE
FM_TSIZE            .equ FM_TCNT + FM_TREE_SIZE
FM_PILE             .equ FM_TSIZE + FM_TREE_SIZE
FM_PILE_SIZE        .equ $200
FM_PLANE            .equ FM_MAX_NODES
FM_SYMW             .equ FM_PILE + FM_PILE_SIZE
FM_SYMF             .equ FM_SYMW + $100
FM_FIRST            .equ FM_SYMF + $100
FM_LAST             .equ FM_FIRST + FM_PLANE
FM_DEPTH            .equ FM_LAST + FM_PLANE
FM_FOLDS             .equ FM_DEPTH + FM_PLANE
FM_LOW_USED         .equ FM_FOLDS + FM_PLANE - FM_LOW
#ifndef FM_LOW_FREE
FM_LOW_FREE         .equ $280
#endif
RAM_DISJOINT(FM_HDR, FM_HDR_SIZE, KSTACK, KSTACK_TOP - KSTACK, "the statement headers must stay off the kernel's stack page, which a refused kernel call writes")
#if FM_LOW_USED > FM_LOW_SIZE
 .error "FM_LOW_SIZE in ram_map.inc is smaller than what fm lays out"
#endif
#if (FM_LOW_USED + FM_LOW_FREE) > FM_LOW_SIZE
 .error "fm must leave $280 bytes of FM_LOW unused"
#endif

#define FM_NEXT 0
#macro FM_VAR(name, offset, size)
RAM_VAR(name, FM_RAM, offset, size, FM_VARS_SIZE, FM_NEXT, "the fm variables overlap, leave a gap, or run past FM_VARS_SIZE")
#define FM_NEXT eval(offset + size)
#endmacro
#macro FM_AT(offset)
RAM_AT(offset, FM_VARS_SIZE, FM_NEXT, "FM_AT must not go back or past FM_VARS_SIZE in fm/ram.inc")
#define FM_NEXT eval(offset)
#endmacro

FM_VAR(fv_open, $00, 2)
FM_VAR(fv_sp, $02, 2)
FM_VAR(fv_p5, $04, 1)
FM_VAR(fv_iff, $05, 1)
FM_VAR(fv_glen, $06, 2)
FM_VAR(fv_nsyms, $08, 1)
FM_VAR(fv_tok, $09, 2)
FM_VAR(fv_count, $0B, 1)
FM_VAR(fv_hdr, $0C, 2)
FM_VAR(fv_nhyps, $0E, 1)
FM_VAR(fv_last, $0F, 1)
FM_VAR(fv_nw, $10, 1)
FM_VAR(fv_ntok, $11, 2)
FM_VAR(fv_tokp, $13, 2)
FM_VAR(fv_nused, $15, 2)
FM_VAR(fv_np, $17, 2)
FM_VAR(fv_k, $19, 1)
FM_VAR(fv_cn, $1A, 2)
FM_VAR(fv_src, $1C, 3)
FM_VAR(fv_left, $1F, 2)
FM_VAR(fv_dst, $21, 2)
FM_VAR(fv_limit, $23, 1)
FM_VAR(fv_i, $24, 1)
FM_VAR(fv_line, $25, 1)
FM_VAR(fv_prev, $26, 1)
FM_VAR(fv_draw, $27, 1)
FM_VAR(fv_x0, $28, 1)
FM_VAR(fv_row, $29, 2)
FM_VAR(fv_lo, $2B, 1)
FM_VAR(fv_hi, $2C, 1)
FM_VAR(fv_show, $2D, 1)
FM_VAR(fv_rowb, $2E, 1)
FM_VAR(fv_j, $2F, 1)
FM_VAR(fv_seen, $30, 1)
FM_VAR(fv_tmp, $31, 1)
FM_VAR(fv_end, $32, 2)
FM_VAR(fv_byte, $34, 1)
FM_VAR(fv_lib, $35, 6)
FM_VAR(fv_total, $3B, 1)
FM_VAR(fv_ts, $3C, 3)
FM_VAR(fv_tl, $3F, 2)
FM_VAR(fv_tleft, $41, 2)
FM_VAR(fv_tp, $43, 2)
FM_VAR(fv_tc, $45, 1)
FM_VAR(fv_ti, $46, 1)
FM_VAR(fv_pile, $47, 2)
FM_VAR(fv_cur, $49, 1)
FM_VAR(fv_pos, $4A, 1)
FM_VAR(fv_dep, $4B, 1)
FM_VAR(fv_nrow, $4C, 1)
FM_VAR(fv_shp, $4D, 2)
FM_VAR(fv_shn, $4F, 1)
FM_VAR(fv_ns, $50, 2)
FM_VAR(fv_shl, $52, 2)
FM_VAR(fv_te, $54, 2)
FM_VAR(fv_si, $56, 2)
FM_VAR(fv_so, $58, 2)
FM_VAR(fv_ar, $5A, 1)
FM_VAR(fv_sk, $5B, 6)
FM_VAR(fv_x, $61, 2)
FM_VAR(fv_gap, $63, 1)
FM_VAR(fv_id, $64, 1)
FM_VAR(fv_ip, $65, 2)
FM_VAR(fv_consts, $67, 1)
FM_VAR(fv_ew, $68, 1)
FM_VAR(fv_ell, $69, 2)
FM_VAR(fv_then, $6B, 2)
FM_VAR(fv_fn, $6D, 1)
FM_VAR(fv_fm, $6E, 1)
FM_VAR(fv_tb, $6F, 2)
FM_VAR(fv_tcnt, $71, 1)
FM_VAR(fv_m, $72, 1)
FM_VAR(fv_md, $73, 1)
FM_VAR(fv_ni, $74, 1)
FM_VAR(fv_fa, $75, 1)
FM_VAR(fv_fb, $76, 1)
FM_VAR(fv_ta, $77, 1)
FM_VAR(fv_cf, $78, 1)
FM_VAR(fv_cl, $79, 1)
FM_VAR(fv_cptr, $7A, 2)
FM_VAR(fv_cs, $7C, 1)
FM_VAR(fv_ce, $7D, 1)
FM_VAR(fv_ca, $7E, 1)
FM_VAR(fv_cb, $7F, 1)
FM_VAR(fv_xs, $80, 1)
FM_VAR(fv_xe, $81, 1)
FM_VAR(fv_curn, $82, 1)
FM_VAR(fv_cc, $83, 1)
FM_VAR(fv_room, $84, 1)

FM_OPEN_MAGIC       .equ $464D
FM_OWNED            .equ $4F46
#if (FM_OWNED == READY_MAGIC) || (FM_OWNED == 0)
 .error "FM_OWNED must differ from the kernel's ready magic and from 0"
#endif
FM_SPACE            .equ 2
FM_ELLIPSIS         .equ 7
FM_PAD_VAR          .equ 4
FM_MODE_TOGGLE      .equ 2

FM_OK               .equ 0
FM_ERR_NOT_OPEN     .equ 1
FM_ERR_CLOSED       .equ 2
FM_ERR_NO_MAP       .equ 3
FM_ERR_FONT         .equ 4
FM_ERR_TOO_BIG      .equ 5
FM_ERR_BAD          .equ 6
FM_ERR_INDEX        .equ 7
FM_ERR_READ         .equ 8
FM_ERR_TREE         .equ 9
FM_ERR_LIMIT        .equ 10
