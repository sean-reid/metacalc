#include "kernel/ram.inc"
#if FM_RAM & $FF
 .error "FM_RAM must be a multiple of $100"
#endif

#ifndef FM_VARS_SIZE
FM_VARS_SIZE        .equ $26
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
FM_IDS              .equ FM_SECOND + FM_SECOND_SIZE
FM_WIDTHS           .equ FM_IDS + $100
FM_GAPS             .equ FM_WIDTHS + $100
FM_FLAGS            .equ FM_GAPS + $100
FM_RAM_USED         .equ FM_FLAGS + $100 - FM_RAM
#if FM_RAM_USED > FM_RAM_SIZE
 .error "FM_RAM_SIZE in ram_map.inc is smaller than what fm lays out"
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
FM_VAR(fv_count, $09, 1)
FM_VAR(fv_k, $0A, 1)
FM_VAR(fv_cn, $0B, 2)
FM_VAR(fv_src, $0D, 3)
FM_VAR(fv_left, $10, 2)
FM_VAR(fv_dst, $12, 2)
FM_VAR(fv_limit, $14, 1)
FM_VAR(fv_i, $15, 1)
FM_VAR(fv_line, $16, 1)
FM_VAR(fv_prev, $17, 1)
FM_VAR(fv_draw, $18, 1)
FM_VAR(fv_x0, $19, 1)
FM_VAR(fv_row, $1A, 2)
FM_VAR(fv_lo, $1C, 1)
FM_VAR(fv_hi, $1D, 1)
FM_VAR(fv_show, $1E, 1)
FM_VAR(fv_rowb, $1F, 1)
FM_VAR(fv_j, $20, 1)
FM_VAR(fv_seen, $21, 1)
FM_VAR(fv_tmp, $22, 1)
FM_VAR(fv_end, $23, 2)
FM_VAR(fv_byte, $25, 1)

FM_OPEN_MAGIC       .equ $464D
FM_SPACE            .equ 2

FM_OK               .equ 0
FM_ERR_NOT_OPEN     .equ 1
FM_ERR_CLOSED       .equ 2
FM_ERR_NO_MAP       .equ 3
FM_ERR_FONT         .equ 4
FM_ERR_TOO_BIG      .equ 5
FM_ERR_BAD          .equ 6
FM_ERR_INDEX        .equ 7
FM_ERR_READ         .equ 8
