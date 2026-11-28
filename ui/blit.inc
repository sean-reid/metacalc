; ui_blit   copies the frame to the LCD through ports 10h and 11h, one column of bytes at a time
;           in 8 bit mode with the row counter incrementing; before each write it waits for the
;           busy flag, bit 7 of port 10h, to clear, so it holds at 6 and 15 MHz. Each wait gives
;           up after UI_BUSY_LIMIT polls, 8,925 T-states against a busy window of 50 clocks in the
;           emulator, and writes anyway, so a flag stuck busy costs pixels, not a hang, but each
;           blit then takes about 7.1 million T-states, 1.2 s at 6 MHz. The controller is left
;           in 8 bit mode with increment mode 5. Interrupts are off for the whole call and come
;           back as the caller had them: a handler that wrote the LCD mid-column would put
;           the rest of the column elsewhere.
;           destroys AF BC DE HL

#macro WAIT
	ld b,UI_BUSY_LIMIT
	in f,(c)
	jp p,$+5
	djnz $-5
#endmacro

; two NOPs put the first poll just after the controller's busy window ends
#macro BYTE
	ld a,(hl)
	add hl,de
	nop
	nop
	WAIT
	out (LCD_DATA),a
#endmacro

ui_blit:
	ld a,i
	jp pe,bl_on
	ld a,i
	jp pe,bl_on
	xor a
	jr bl_go
bl_on:
	ld a,1
bl_go:
	ld (ui_iff),a
bl_mask:
	di
	ld c,LCD_CMD
	WAIT
	ld a,$01
	out (LCD_CMD),a
	WAIT
	ld a,$05
	out (LCD_CMD),a
	ld hl,ui_dirty
	ld a,(hl)
	ld (hl),0
	ld (ui_sent),a
	ld (ui_col),a
	or a
	jp z,bl_done
	ld hl,UI_FRAME
	ld de,UI_STRIDE
	ld a,$20
	ld (ui_cnt),a
bl_col:
	WAIT
	ld a,(ui_cnt)
	out (LCD_CMD),a
	ld a,$C0
	ld (ui_blk),a
bl_stale:
	ld a,(ui_col)
	rrca
	ld (ui_col),a
	jr c,bl_row
bl_skip:
	ld a,l
	add a,UI_ROW*UI_STRIDE
	ld l,a
	jr nc,bl_step
	inc h
bl_step:
	ld a,(ui_blk)
	add a,8
	ld (ui_blk),a
	jr nz,bl_stale
bl_end:
	ld bc,1-UI_FRAME_SIZE
	add hl,bc
	ld c,LCD_CMD
	ld a,(ui_cnt)
	inc a
	ld (ui_cnt),a
	cp $20+UI_STRIDE
	jp nz,bl_col
bl_done:
	ld a,(ui_iff)
	or a
	ret z
	ei
	ret
bl_row:
	WAIT
	ld a,(ui_blk)
	sub $40
	out (LCD_CMD),a
	jr bl_send
bl_fresh:
	ld a,(ui_col)
	rrca
	ld (ui_col),a
	jp nc,bl_skip
bl_send:
	BYTE
	BYTE
	BYTE
	BYTE
	BYTE
	BYTE
	BYTE
	BYTE
	ld a,(ui_blk)
	add a,8
	ld (ui_blk),a
	jp nz,bl_fresh
	jp bl_end
ui_blit_end:
