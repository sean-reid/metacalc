; ui_blit   copies the text rows of the frame whose bits are set in ui_dirty to the LCD through ports
;           10h and 11h, one column of bytes at a time in 8 bit mode with the row counter incrementing,
;           then stores that mask in ui_sent and 0 in ui_dirty; a row not in it is not sent and the LCD
;           keeps what it has. Per column it sends the command 20h + column, then for each run of
;           adjacent rows the row command 80h + 8r once and the eight bytes of each row; a mask of $FF
;           is one run, and a mask of 0 sends the two mode commands and nothing else. Before each
;           write it waits for the busy flag, bit 7 of port 10h, to clear, so it holds at 6 and 15 MHz.
;           Each wait gives up after UI_BUSY_LIMIT polls, 8,925 T-states against a busy window of 50
;           clocks in the emulator, and writes anyway, so a flag stuck busy costs pixels, not a hang,
;           but each blit then takes about 7.1 million T-states, 1.2 s at 6 MHz. The controller is left
;           in 8 bit mode with increment mode 5. Interrupts are off for the whole call and come back as
;           the caller had them, on every exit: a handler that wrote the LCD mid-column would put the
;           rest of the column elsewhere, and a handler must not call the canvas, since the call
;           overwrites the scratch bytes. ui_dirty and ui_sent hold nothing across an auto power down.
;           Once per call it builds the list of the mask's runs, at most four, in the scratch bytes
;           at UI_VARS offsets 00h to 0Fh (the text and soft key scratch, which hold nothing between
;           calls): four bytes a run, the block count as -8 times the blocks, the row command, and the
;           16 bit step to add to the frame pointer after the run, 96 a skipped row, or for the last
;           run back to the first run's start in the next byte column, which is the only step that
;           sets carry. It also uses ui_kb+1 and ui_kn (10h, 11h), ui_blk, ui_sp and ui_iff. SP points
;           into the list while the columns are sent and is restored from ui_sp on every exit, so the
;           stack needed is the return address only: 2 bytes.
;           destroys AF BC DE HL

#if ((UI_VARS + 16) > (ui_kb + 1)) || ((ui_kb + 3) > ui_dirty) || ((ui_kn + 1) > ui_dirty)
 .error "the blit's list and its scratch must lie in UI_VARS below ui_dirty"
#endif

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
	ld (ui_sp),sp
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
	or a
	jp z,bl_done
	ld c,a
	ld hl,UI_FRAME
	ld de,UI_ROW*UI_STRIDE
	ld a,$80
bl_head:
	srl c
	jr c,bl_first
	add a,8
	add hl,de
	jr bl_head
bl_first:
	ld (ui_kb+1),hl
	ld de,UI_VARS+1
bl_new:
	ld (de),a
	dec de
	ld b,a
bl_len:
	add a,8
	srl c
	jr c,bl_len
	ld l,a
	ld a,b
	sub l
	ld (de),a
	inc c
	dec c
	jr z,bl_last
	inc de
	inc de
	ld a,l
bl_gap:
	add a,8
	srl c
	jr nc,bl_gap
	ld b,a
	sub l
	ld l,a
	add a,a
	add a,l
	ld l,a
	ld h,0
	add hl,hl
	add hl,hl
	ld a,l
	ld (de),a
	inc de
	ld a,h
	ld (de),a
	inc de
	inc de
	ld a,b
	jr bl_new
bl_last:
	inc de
	inc de
	ld a,l
	ld hl,UI_VARS+1
	sub (hl)
	ld l,a
	add a,a
	add a,l
	ld l,a
	ld h,0
	add hl,hl
	add hl,hl
	xor a
	sub l
	ld l,a
	ld a,0
	sbc a,h
	ld h,a
	inc hl
	ld a,l
	ld (de),a
	inc de
	ld a,h
	ld (de),a
	ld hl,(ui_kb+1)
	ld a,$20
	ld (ui_kn),a
	ld de,UI_STRIDE
	ld c,LCD_CMD
bl_col:
	WAIT
	ld a,(ui_kn)
	out (LCD_CMD),a
	ld sp,UI_VARS
bl_run:
	pop bc
	ld a,c
	ld (ui_blk),a
	ld a,b
	ld c,LCD_CMD
	WAIT
	out (LCD_CMD),a
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
	jp nz,bl_send
	pop de
	add hl,de
	ld de,UI_STRIDE
	jp nc,bl_run
	ld a,(ui_kn)
	inc a
	ld (ui_kn),a
	cp $20+UI_STRIDE
	jp nz,bl_col
bl_done:
	ld sp,(ui_sp)
	ld a,(ui_iff)
	or a
	ret z
	ei
	ret
ui_blit_end:
