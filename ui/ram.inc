#ifndef UI_WIDTH
UI_WIDTH                .equ 96
#endif
UI_HEIGHT               .equ 64
UI_ROW                  .equ 8
UI_TEXT_ROWS            .equ 8
UI_STRIDE               .equ 12
UI_MARGIN               .equ 1
UI_SOFT_ROW             .equ 7
UI_BUSY_LIMIT           .equ 255

#include "ram_map.inc"
#if (UI_FRAME_SIZE < (UI_STRIDE * UI_HEIGHT)) || (UI_FRAME_SIZE > (UI_STRIDE * UI_HEIGHT))
 .error "UI_FRAME_SIZE in ram_map.inc must be UI_STRIDE*UI_HEIGHT"
#endif

#define UI_NEXT 0
#macro UI_VAR(name, offset, size)
RAM_VAR(name, UI_VARS, offset, size, UI_VARS_SIZE, UI_NEXT, "the UI variables overlap, leave a gap, or run past UI_VARS_SIZE in ram_map.inc")
#define UI_NEXT eval(offset + size)
#endmacro
#macro UI_AT(offset)
RAM_AT(offset, UI_VARS_SIZE, UI_NEXT, "UI_AT must not go back or past UI_VARS_SIZE in ram_map.inc")
#define UI_NEXT eval(offset)
#endmacro

UI_VAR(ui_rowbase, $00, 2)
UI_VAR(ui_dst, $02, 2)
UI_VAR(ui_src, $04, 2)
UI_VAR(ui_shift, $06, 1)
UI_VAR(ui_narrow, $07, 1)
UI_VAR(ui_col, $08, 1)
UI_VAR(ui_cnt, $09, 1)
UI_VAR(ui_lm, $0A, 1)
UI_VAR(ui_rm, $0B, 1)
UI_VAR(ui_mid, $0C, 1)
UI_VAR(ui_kp, $0D, 2)
UI_VAR(ui_kb, $0F, 2)
UI_VAR(ui_kn, $11, 1)
UI_AT($14)
UI_VAR(ui_fontp, $14, 2)
UI_VAR(ui_sp, $16, 2)
UI_VAR(ui_iff, $18, 1)
UI_VAR(ui_blk, $19, 1)

SLOT                    .equ 8
SPACE_CODE              .equ 1
LCD_CMD                 .equ $10
LCD_DATA                .equ $11
