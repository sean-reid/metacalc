; Kind 2, a notice as crates/mcui/src/screens/notice.rs draws it: the title bar, one line centred
; on row 3 and Back over F1. The frame's byte 1 is the notice's number and byte 2 its parameter.
;   NOTICE_THEOREMS  "Theorems", "Not available yet"
;   NOTICE_VERIFY    "Verify", "Not available yet"
;   NOTICE_ERROR     "Error", "Status " and the parameter as two hex digits, built in APP_BUF by
;                    notice_enter, since the OS overwrites saveSScreen at an auto power down
; F1, CLEAR and ENTER pop; every other key stays.

NOTICE_THEOREMS	.equ 0
NOTICE_VERIFY	.equ 1
NOTICE_ERROR	.equ 2
NOTICE_ROW	.equ 3

notice_titles:
	.dw str_theorems, str_verify, str_error
notice_lines:
	.dw str_unavailable, str_unavailable, APP_BUF

notice_status:
	.db UI_FONT_CH_53, UI_FONT_CH_74, UI_FONT_CH_61, UI_FONT_CH_74, UI_FONT_CH_75, UI_FONT_CH_73, UI_FONT_CH_20

; In: HL = a frame. Builds the error notice's line. Out: A = 0.
notice_enter:
	inc hl
	ld a, (hl)
	cp NOTICE_ERROR
	jr nz, notice_built
	inc hl
	ld a, (hl)
	push af
	ld hl, notice_status
	ld de, APP_BUF
	ld bc, 7
	ldir
	ex de, hl
	pop af
	call app_hex_byte
notice_built:
	xor a
	ret

; In: HL = a frame.
notice_draw:
	push hl
	call far_ui_clear
	pop hl
	inc hl
	ld a, (hl)
	add a, a
	ld c, a
	ld b, 0
	ld hl, notice_titles
	add hl, bc
	ld e, (hl)
	inc hl
	ld d, (hl)
	ex de, hl
	ld de, str_none
	push bc
	call far_ui_title
	pop bc
	ld hl, notice_lines
	add hl, bc
	ld e, (hl)
	inc hl
	ld d, (hl)
	ex de, hl
	ld b, NOTICE_ROW
	call far_ui_text_centered
	ld hl, str_keys_back
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
