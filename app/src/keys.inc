; The app's one key wait: app_key waits for a key with the OS's _GetKey, sets the canvas font
; again, and returns the model's key (crates/mcui/src/key.rs). Every call into the OS that waits
; for a key is here, so this file can sit on the app's entry page.
;
; Included after equates.inc and ui.z80, which give _GetKey, onFlags, ui_set_font and ui_font.
;
; app_key   out: A = the key, one of the KEY_ codes below; KEY_NONE for a key the app ignores
;                B = for KEY_LETTER, the letter as an uppercase ASCII code; 0 for every other key
;                destroys every register but IY, which must hold the OS flags pointer; the
;                shadow registers and IX are the OS's to change. Returns with interrupts on.
;
; _GetKey handles 2nd and ALPHA, so a letter arrives in one of two forms: the key's own code, or
; a capital's code after ALPHA or alpha lock. Both give KEY_LETTER, so the key that carries a
; letter needs no ALPHA press. The keys that also carry a digit give their letter, so the digits
; cannot be typed. 2nd and ALPHA return nothing; the other 2nd keys return their own codes and
; give KEY_NONE, except 2nd+QUIT, which gives KEY_QUIT.
;
; ON is never returned. The OS sets onInterrupt when ON goes down and _GetKey then returns 0 at
; once, on every call, until the flag is clear. app_key clears the flag, waits for ON to come up
; and waits for a key again, so a press leaves no break for the next long job.
;
; The OS zeroes UI_VARS when auto power down fires inside _GetKey, and the font pointer is the one
; byte of it that holds anything between calls; app_key sets the canvas font after every wait,
; power down or not. 2nd+OFF never returns: the OS leaves the app, and the app is not entered
; again when the calculator is turned back on.

KEY_NONE         .equ 0
KEY_UP           .equ 1
KEY_DOWN         .equ 2
KEY_LEFT         .equ 3
KEY_RIGHT        .equ 4
KEY_ENTER        .equ 5
KEY_CLEAR        .equ 6
KEY_F1           .equ 7
KEY_F2           .equ 8
KEY_F3           .equ 9
KEY_F4           .equ 10
KEY_F5           .equ 11
KEY_LETTER       .equ 12
KEY_QUIT         .equ 13

KEY_CAP_A        .equ $9A
KEY_PAIRS        .equ 12

app_key:
ak_wait:
	ei
	bcall(_GetKey)
	bit onInterrupt,(iy+onFlags)
	jr nz,ak_on
	ld c,a
	ld hl,ui_font
	call ui_set_font
	ld b,0
	ld a,c
	ld hl,ak_pairs
	ld d,KEY_PAIRS
ak_pair:
	cp (hl)
	inc hl
	jr z,ak_hit
	inc hl
	dec d
	jr nz,ak_pair
	sub KEY_CAP_A
	cp 26
	jr c,ak_letter
	ld a,c
	ld hl,ak_plain
	ld d,26
ak_plain_next:
	cp (hl)
	inc hl
	jr z,ak_plain_hit
	dec d
	jr nz,ak_plain_next
	xor a
	ret
ak_plain_hit:
	ld a,26
	sub d
ak_letter:
	add a,'A'
	ld b,a
	ld a,KEY_LETTER
	ret
ak_hit:
	ld a,(hl)
	ret
ak_on:
	ei
	halt
	in a,(4)
	bit 3,a
	jr z,ak_on
	res onInterrupt,(iy+onFlags)
	jr ak_wait

; The OS's code for a key and the code it gives.
ak_pairs:
	.db $03,KEY_UP
	.db $04,KEY_DOWN
	.db $02,KEY_LEFT
	.db $01,KEY_RIGHT
	.db $05,KEY_ENTER
	.db $09,KEY_CLEAR
	.db $49,KEY_F1
	.db $48,KEY_F2
	.db $2E,KEY_F3
	.db $5A,KEY_F4
	.db $44,KEY_F5
	.db $40,KEY_QUIT

; The OS's code for the key that carries each letter, A to Z, before any ALPHA.
ak_plain:
	.db $32,$2C,$2D,$B6,$B7,$B9,$BB,$84,$BD,$8B,$85,$86,$83
	.db $C1,$95,$96,$97,$82,$BF,$92,$93,$94,$81,$8A,$8F,$90
