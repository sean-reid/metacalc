; probe_blit is ui_blit's instruction sequence, copied, with a count of the busy reads of every
; wait added right after the wait's last poll, so the gap from an LCD write to the next first poll is
; the same instructions as in ui_blit. Built with REF_BLIT the probe calls ui_blit itself instead,
; which is how a test compares the two.
; Contract: as ui_blit (ui/blit.inc): interrupts off for the call, back as the caller had them,
; AF BC DE HL destroyed, the controller left in 8 bit mode with increment mode 5. It also uses the
; shadow registers, and adds to the counters below.
; Counters, 16 bit little endian words: LCD_HIST_DATA holds four, the writes to the data port that
; read busy 0, 1, 2 and 3 or more times before the controller was ready; LCD_HIST_CMD the same for
; the writes to the command port; LCD_TOTAL_DATA and LCD_TOTAL_CMD the busy reads of each, which stop
; at 65535. A wait that reads busy UI_BUSY_LIMIT times and gives up counts that many.

; The busy reads of the wait just made: UI_BUSY_LIMIT less B, which counts down once per busy read.
#macro PCOUNT(hist, total)
	ex af,af'
	ld a,UI_BUSY_LIMIT
	sub b
	exx
	ld c,a
	cp 3
	jr c,$+4
	ld a,3
	add a,a
	ld e,a
	ld d,0
	ld hl,hist
	add hl,de
	inc (hl)
	jr nz,$+4
	inc hl
	inc (hl)
	ld hl,(total)
	ld b,0
	add hl,bc
	jr nc,$+5
	ld hl,0FFFFh
	ld (total),hl
	exx
	ex af,af'
#endmacro

#macro PWAIT(hist, total)
	ld b,UI_BUSY_LIMIT
	in f,(c)
	jp p,$+5
	djnz $-5
	PCOUNT(hist, total)
#endmacro

#macro PBYTE
	ld a,(hl)
	add hl,de
	nop
	nop
	PWAIT(LCD_HIST_DATA, LCD_TOTAL_DATA)
	out (LCD_DATA),a
#endmacro

probe_blit:
	ld a,i
	jp pe,pb_on
	ld a,i
	jp pe,pb_on
	xor a
	jr pb_go
pb_on:
	ld a,1
pb_go:
	ld (ui_iff),a
	di
	ld c,LCD_CMD
	PWAIT(LCD_HIST_CMD, LCD_TOTAL_CMD)
	ld a,$01
	out (LCD_CMD),a
	PWAIT(LCD_HIST_CMD, LCD_TOTAL_CMD)
	ld a,$05
	out (LCD_CMD),a
	ld hl,UI_FRAME
	ld de,UI_STRIDE
	ld a,$20
	ld (ui_cnt),a
pb_col:
	PWAIT(LCD_HIST_CMD, LCD_TOTAL_CMD)
	ld a,(ui_cnt)
	out (LCD_CMD),a
	PWAIT(LCD_HIST_CMD, LCD_TOTAL_CMD)
	ld a,$80
	out (LCD_CMD),a
	ld a,UI_HEIGHT/8
	ld (ui_blk),a
pb_block:
	PBYTE
	PBYTE
	PBYTE
	PBYTE
	PBYTE
	PBYTE
	PBYTE
	PBYTE
	ld a,(ui_blk)
	dec a
	ld (ui_blk),a
	jp nz,pb_block
	ld bc,1-UI_FRAME_SIZE
	add hl,bc
	ld c,LCD_CMD
	ld a,(ui_cnt)
	inc a
	ld (ui_cnt),a
	cp $20+UI_STRIDE
	jp nz,pb_col
	ld a,(ui_iff)
	or a
	ret z
	ei
	ret
