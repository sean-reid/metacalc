; Kind 4, the About screen of a stage as About::stage in crates/mcui/src/screens/about.rs draws it:
; the stage's name in the title bar, its title on row 2, and on rows 4 to 6 "N assertions",
; "N checked" and "N failed", centred. The numbers are the stage's, from the same counts the
; Libraries screen shows (screen_counts.inc): the assertions up to its end, n1 + n3 of the store
; and n2. Frame: byte 1 the stage. A build without LIBRARY never pushes it. The name and the title
; are cut to the width of the screen as the canvas cuts them. CLEAR and ENTER pop (about_key);
; every other key stays.
;
; stage_enter  out: A = the status of libs_counts, which derives the counts the draw reads.
; stage_draw   builds each string in APP_TEXT and draws it.

STAGE_BUF	.equ 49
STAGE_NUM	.equ 24
STAGE_WIDTH	.equ 94

SG_IDX		.equ APP_TEXT
SG_EMPTY	.equ APP_TEXT + 1
SG_TOTAL	.equ APP_TEXT + 2
SG_CHECKED	.equ APP_TEXT + 4
SG_FAILED	.equ APP_TEXT + 6
SG_RAW		.equ APP_TEXT + 8
SG_CUT		.equ SG_RAW + STAGE_BUF
SG_NUM		.equ SG_CUT + STAGE_BUF
#if (SG_NUM + STAGE_NUM) > (APP_TEXT + APP_TEXT_SIZE)
 .error "the stage About screen's variables are longer than APP_TEXT"
#endif

#ifdef LIBRARY
stage_enter:
	jp libs_counts

stage_draw:
	inc hl
	ld a, (hl)
	ld (SG_IDX), a
	call far_ui_clear
	xor a
	ld (SG_EMPTY), a
	ld a, (SG_IDX)
	call libs_figures
	ld (SG_TOTAL), hl
	ld (SG_CHECKED), de
	ld (SG_FAILED), bc
	ld a, (SG_IDX)
	call stage_strings
	push de
	call stage_cut
	ld hl, SG_CUT
	ld de, SG_EMPTY
	call far_ui_title
	pop hl
	call stage_cut
	ld hl, SG_CUT
	ld b, 2
	call far_ui_text_centered
	ld hl, (SG_TOTAL)
	ld de, stage_assertions
	ld b, 4
	call stage_line
	ld hl, (SG_CHECKED)
	ld de, stage_checked
	ld b, 5
	call stage_line
	ld hl, (SG_FAILED)
	ld de, stage_failed
	ld b, 6
	jp stage_line

; In: HL = a string of the stage table. Writes it to SG_CUT cut to the width of the screen.
stage_cut:
	ld de, SG_RAW
	call app_strcpy
	ld hl, SG_RAW
	ld de, SG_CUT
	ld b, STAGE_BUF
	ld c, STAGE_WIDTH
	jp far_ui_truncate

; In: HL = a number, DE = the text after it, B = the row. Draws them centred.
stage_line:
	push bc
	push de
	ld de, SG_NUM
	call app_dec16
	pop hl
	call app_strcpy
	ld hl, SG_NUM
	pop bc
	jp far_ui_text_centered

stage_assertions:
	.db UI_FONT_CH_20, UI_FONT_CH_61, UI_FONT_CH_73, UI_FONT_CH_73, UI_FONT_CH_65, UI_FONT_CH_72, UI_FONT_CH_74, UI_FONT_CH_69, UI_FONT_CH_6F, UI_FONT_CH_6E, UI_FONT_CH_73, 0
stage_checked:
	.db UI_FONT_CH_20, UI_FONT_CH_63, UI_FONT_CH_68, UI_FONT_CH_65, UI_FONT_CH_63, UI_FONT_CH_6B, UI_FONT_CH_65, UI_FONT_CH_64, 0
stage_failed:
	.db UI_FONT_CH_20, UI_FONT_CH_66, UI_FONT_CH_61, UI_FONT_CH_69, UI_FONT_CH_6C, UI_FONT_CH_65, UI_FONT_CH_64, 0
#else
stage_enter:
stage_draw:
	xor a
	ret
#endif
