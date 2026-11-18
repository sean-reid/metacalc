; ui_blit   copies the frame to the LCD through ports 10h and 11h, one column of bytes at a time
;           in 8 bit mode with the row counter incrementing; before each write it waits for the
;           busy flag, bit 7 of port 10h, to clear, so it holds at 6 and 15 MHz. The controller
;           is left in 8 bit mode with the row counter incrementing.
;           destroys AF BC DE HL

#macro WAIT
	in f,(c)
	jp m,$-2
#endmacro

ui_blit:
	ld c,LCD_CMD
	WAIT
	ld a,$01
	out (LCD_CMD),a
	WAIT
	ld a,$05
	out (LCD_CMD),a
	ld hl,UI_FRAME
	ld de,UI_STRIDE
	ld a,$20
	ld (ui_cnt),a
bl_col:
	WAIT
	ld a,(ui_cnt)
	out (LCD_CMD),a
	WAIT
	ld a,$80
	out (LCD_CMD),a
	ld b,UI_HEIGHT
bl_byte:
	ld a,(hl)
	add hl,de
	nop
	WAIT
	out (LCD_DATA),a
	djnz bl_byte
	ld bc,1-UI_FRAME_SIZE
	add hl,bc
	ld c,LCD_CMD
	ld a,(ui_cnt)
	inc a
	ld (ui_cnt),a
	cp $20+UI_STRIDE
	jr nz,bl_col
	ret
ui_blit_end:
