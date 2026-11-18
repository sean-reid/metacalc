#ifndef UI_RAM
UI_RAM                  .equ $9000
#endif
UI_RAM_SIZE             .equ $0330
#if UI_RAM < $8000
 .error "UI_RAM must be in bank B"
#endif
#if UI_RAM + UI_RAM_SIZE > $C000
 .error "UI_RAM must leave $330 bytes inside bank B"
#endif

UI_WIDTH                .equ 96
UI_HEIGHT               .equ 64
UI_ROW                  .equ 8
UI_STRIDE               .equ 12
UI_MARGIN               .equ 1
UI_SOFT_ROW             .equ 7

UI_FRAME                .equ UI_RAM+$000
UI_FRAME_SIZE           .equ UI_STRIDE*UI_HEIGHT
ui_rowbase              .equ UI_RAM+$300
ui_dst                  .equ UI_RAM+$302
ui_src                  .equ UI_RAM+$304
ui_shift                .equ UI_RAM+$306
ui_narrow               .equ UI_RAM+$307
ui_col                  .equ UI_RAM+$308
ui_cnt                  .equ UI_RAM+$309
ui_lm                   .equ UI_RAM+$30A
ui_rm                   .equ UI_RAM+$30B
ui_mid                  .equ UI_RAM+$30C
ui_rows                 .equ UI_RAM+$30D
ui_kp                   .equ UI_RAM+$30E
ui_kb                   .equ UI_RAM+$310
ui_kn                   .equ UI_RAM+$312
ui_fontp                .equ UI_RAM+$314
ui_sp                   .equ UI_RAM+$316
ui_iff                  .equ UI_RAM+$318

SLOT                    .equ 8
SPACE_CODE              .equ 1
LCD_CMD                 .equ $10
LCD_DATA                .equ $11
