UI_WIDTH                .equ 96
UI_HEIGHT               .equ 64
UI_ROW                  .equ 8
UI_STRIDE               .equ 12
UI_MARGIN               .equ 1
UI_SOFT_ROW             .equ 7

#ifndef UI_FRAME
UI_FRAME                .equ $9340
#endif
#ifndef UI_VARS
UI_VARS                 .equ $9882
#endif
UI_FRAME_SIZE           .equ UI_STRIDE*UI_HEIGHT
UI_VARS_SIZE            .equ $19
#if (UI_FRAME + UI_FRAME_SIZE) > $10000
 .error "UI_FRAME must leave 768 bytes"
#endif
#if (UI_VARS + UI_VARS_SIZE) > $10000
 .error "UI_VARS must leave 25 bytes"
#endif
#if UI_FRAME < (UI_VARS + UI_VARS_SIZE)
#if UI_VARS < (UI_FRAME + UI_FRAME_SIZE)
 .error "UI_FRAME and UI_VARS overlap"
#endif
#endif

ui_rowbase              .equ UI_VARS+$00
ui_dst                  .equ UI_VARS+$02
ui_src                  .equ UI_VARS+$04
ui_shift                .equ UI_VARS+$06
ui_narrow               .equ UI_VARS+$07
ui_col                  .equ UI_VARS+$08
ui_cnt                  .equ UI_VARS+$09
ui_lm                   .equ UI_VARS+$0A
ui_rm                   .equ UI_VARS+$0B
ui_mid                  .equ UI_VARS+$0C
ui_rows                 .equ UI_VARS+$0D
ui_kp                   .equ UI_VARS+$0E
ui_kb                   .equ UI_VARS+$10
ui_kn                   .equ UI_VARS+$12
ui_fontp                .equ UI_VARS+$14
ui_sp                   .equ UI_VARS+$16
ui_iff                  .equ UI_VARS+$18

SLOT                    .equ 8
SPACE_CODE              .equ 1
LCD_CMD                 .equ $10
LCD_DATA                .equ $11
