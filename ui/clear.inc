; Clear. Assembled in the tail of the ui_tblk page (glyph.inc).

; ui_clear   zeroes the frame with the stack, so interrupts go off for the call and are restored;
;            the frame must not end above $FFFF
;            destroys AF BC DE HL
ui_clear:
	ld a,i
	jp pe,uc_on
	ld a,i
	jp pe,uc_on
	xor a
	jr uc_go
uc_on:
	ld a,1
uc_go:
	ld (ui_iff),a
	di
	ld (ui_sp),sp
	ld sp,UI_FRAME+UI_FRAME_SIZE
	ld hl,0
	ld b,UI_FRAME_SIZE/32
uc_loop:
#macro PUSH4
	push hl
	push hl
	push hl
	push hl
#endmacro
	PUSH4
	PUSH4
	PUSH4
	PUSH4
	djnz uc_loop
	ld sp,(ui_sp)
	ld a,(ui_iff)
	or a
	ret z
	ei
	ret
