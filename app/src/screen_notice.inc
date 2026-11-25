; Kind 2, a notice as crates/mcui/src/screens/notice.rs draws it: the title bar, one line centred
; on row 3, or two on rows 3 and 4, and Back over F1. The frame's byte 1 is the notice's number and byte 2 its parameter.
;   NOTICE_LIST     the stage's name as the title and "Not available yet", what the list's keys with
;                   no screen behind them push; the parameter is the stage, and notice_enter builds
;                   the title in NT_TITLE; the title is the whole name, where the model's list
;                   cuts it to leave room for "checked/total"
;   NOTICE_DAMAGED  "Verify", "Library is damaged" and "Install it again", which the checking
;                   screen replaces itself with when the library's CRC is wrong
;   NOTICE_ERROR    "Error", "Status " and the parameter as two hex digits, built in NT_LINE by
;                   notice_enter, since the OS overwrites saveSScreen at an auto power down
;   NOTICE_PROOFS   "My proofs", "Not available yet"
;   NOTICE_PICK     "Verify", "Pick a library"
; A number past NOTICE_PICK draws the error notice. F1, CLEAR and ENTER pop; every other key stays.

NOTICE_LIST	.equ 0
NOTICE_DAMAGED	.equ 1
NOTICE_ERROR	.equ 2
NOTICE_PROOFS	.equ 3
NOTICE_PICK	.equ 4
NOTICE_LAST	.equ NOTICE_PICK
NOTICE_ROW	.equ 3

NT_TITLE	.equ APP_TEXT
NT_TITLE_SIZE	.equ 17
NT_LINE		.equ NT_TITLE + NT_TITLE_SIZE
NT_LINE_SIZE	.equ 10
NT_END		.equ NT_LINE + NT_LINE_SIZE
#if NT_END > (APP_TEXT + APP_TEXT_SIZE)
 .error "the notice's variables are longer than APP_TEXT"
#endif

notice_titles:
	.dw NT_TITLE, STG_NOTICE_VERIFY, STG_NOTICE_ERROR, STG_NOTICE_PROOFS, STG_NOTICE_VERIFY
notice_lines:
	.dw STG_NOTICE_UNAVAILABLE, STG_NOTICE_DAMAGED, NT_LINE, STG_NOTICE_UNAVAILABLE, STG_NOTICE_PICK
notice_more:
	.dw 0, STG_NOTICE_AGAIN, 0, 0, 0
#if (notice_lines - notice_titles) != (2 * (NOTICE_LAST + 1))
 .error "notice_titles needs a title for every NOTICE_ number"
#endif
#if (notice_more - notice_lines) != (2 * (NOTICE_LAST + 1))
 .error "notice_lines needs a line for every NOTICE_ number"
#endif
#if ($ - notice_more) != (2 * (NOTICE_LAST + 1))
 .error "notice_more needs a second line, or 0, for every NOTICE_ number"
#endif

notice_status:
	.db UI_FONT_CH_53, UI_FONT_CH_74, UI_FONT_CH_61, UI_FONT_CH_74, UI_FONT_CH_75, UI_FONT_CH_73, UI_FONT_CH_20

; In: HL = a frame. Builds the title of NOTICE_LIST or the line of the error notice, which a number
; past the notices also shows. Out: A = 0.
notice_enter:
	inc hl
	ld a, (hl)
	or a
	jr z, notice_stage
	cp NOTICE_ERROR
	jr z, notice_status_line
	cp NOTICE_LAST + 1
	jr c, notice_built
notice_status_line:
	inc hl
	ld a, (hl)
	push af
	ld hl, notice_status
	ld de, NT_LINE
	ld bc, 7
	ldir
	ex de, hl
	pop af
	call app_hex_byte
notice_built:
	xor a
	ret
notice_stage:
#ifdef LIBRARY
	inc hl
	ld a, (hl)
	call stage_strings
	ld de, NT_TITLE
	call app_strcpy
#else
	xor a
	ld (NT_TITLE), a
#endif
	xor a
	ret

; In: HL = a frame.
notice_draw:
	push hl
	call far_ui_clear
	ld hl, notice_block
	ld bc, NOTICE_BLOCK_SIZE
	call app_stage_block
	pop hl
	inc hl
	ld a, (hl)
	cp NOTICE_LAST + 1
	jr c, notice_known
	ld a, NOTICE_ERROR
notice_known:
	add a, a
	ld c, a
	ld b, 0
	ld hl, notice_titles
	add hl, bc
	ld e, (hl)
	inc hl
	ld d, (hl)
	ex de, hl
	ld de, STG_NOTICE_NONE
	push bc
	call far_ui_title
	pop bc
	push bc
	ld hl, notice_lines
	add hl, bc
	ld e, (hl)
	inc hl
	ld d, (hl)
	ex de, hl
	ld b, NOTICE_ROW
	call far_ui_text_centered
	pop bc
	ld hl, notice_more
	add hl, bc
	ld e, (hl)
	inc hl
	ld d, (hl)
	ld a, d
	or e
	jr z, notice_keys
	ex de, hl
	ld b, NOTICE_ROW + 1
	call far_ui_text_centered
notice_keys:
	ld hl, STG_NOTICE_KEYS
	jp far_ui_soft_keys

; In: A = the key.
notice_key:
	cp KEY_F1
	jr z, notice_pop
	cp KEY_CLEAR
	jr z, notice_pop
	cp KEY_ENTER
	jr z, notice_pop
	ld a, SCR_STAY
	ret
notice_pop:
	ld a, SCR_POP
	ret
