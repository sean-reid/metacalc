; The pattern, the LCD read-back and the comparisons of lcdcheck.
; Byte i of a 96 by 64 frame, row-major, 12 bytes a row, is pat(i): 5Ch + 37 * (i mod 256) mod 256,
; with 00h taken as 5Ah and FFh as A5h so that a cleared or filled screen never matches by chance.
; A result slot is four bytes: the count of differing bytes (word), then the row and the byte
; column of the first one, FFh and FFh when there is none. The row is a pixel row, 0 to 63, the
; column a byte of 8 pixels, 0 to 11.

; The including file defines PAT_LCD_ROWS, PAT_LCD_BYTES, SNAP_BYTES, LCD_BUF and the scratch below.

; Contract: A is i mod 256. Returns pat(i) in A; destroys B and C and the flags.
pat:
	ld b,a
	add a,a
	add a,a
	ld c,a
	add a,a
	add a,a
	add a,a
	add a,c
	add a,b
	add a,5Ch
	or a
	jr nz,pat_not_zero
	ld a,5Ah
	ret
pat_not_zero:
	cp 0FFh
	ret nz
	ld a,0A5h
	ret

; Writes pat(i) over the frame at UI_FRAME. Destroys AF BC DE HL.
fill_pattern:
	ld hl,UI_FRAME
	ld de,UI_FRAME_SIZE
	xor a
	ld (LCD_I),a
fp_loop:
	ld a,(LCD_I)
	call pat
	ld (hl),a
	inc hl
	ld a,(LCD_I)
	inc a
	ld (LCD_I),a
	dec de
	ld a,d
	or e
	jr nz,fp_loop
	ret

; Contract: waits for the busy flag to clear, giving up after UI_BUSY_LIMIT reads. Destroys AF and B.
lcd_wait:
	ld b,UI_BUSY_LIMIT
lw_loop:
	in a,(LCD_CMD)
	rlca
	ret nc
	djnz lw_loop
	ret

; Contract: A is a command byte. Waits, then writes it. Destroys AF and B.
lcd_put_cmd:
	push af
	call lcd_wait
	pop af
	out (LCD_CMD),a
	ret

; Contract: DE is the buffer for 768 bytes. Reads the whole LCD back through the driver as the
; frame is laid out, in 8 bit mode with the row counter incrementing: for each of the 12 byte
; columns it sets the column and row 0, makes the dummy read, then reads the 64 rows, each after a
; wait. Interrupts are off for the call and back as the caller had them. Destroys AF BC DE HL.
read_lcd:
	ld a,i
	jp pe,rl_on
	ld a,i
	jp pe,rl_on
	xor a
	jr rl_go
rl_on:
	ld a,1
rl_go:
	ld (LCD_IFF),a
	di
	ld (LCD_PTR),de
	xor a
	ld (LCD_COL),a
	ld a,$01
	call lcd_put_cmd
	ld a,$05
	call lcd_put_cmd
rl_col:
	ld a,(LCD_COL)
	add a,$20
	call lcd_put_cmd
	ld a,$80
	call lcd_put_cmd
	call lcd_wait
	in a,(LCD_DATA)
	ld hl,(LCD_PTR)
	ld de,UI_STRIDE
	ld c,UI_HEIGHT
rl_row:
	call lcd_wait
	in a,(LCD_DATA)
	ld (hl),a
	add hl,de
	dec c
	jr nz,rl_row
	ld hl,(LCD_PTR)
	inc hl
	ld (LCD_PTR),hl
	ld a,(LCD_COL)
	inc a
	ld (LCD_COL),a
	cp UI_STRIDE
	jr nz,rl_col
	ld a,(LCD_IFF)
	or a
	ret z
	ei
	ret

; Contract: IX is a result slot, HL the first of DE bytes to compare with pat(0), pat(1), and so
; on. Fills the slot. Destroys AF BC DE HL.
cmp_pat:
	call slot_clear
cp_loop:
	ld a,(LCD_I)
	call pat
	cp (hl)
	call nz,note_diff
	inc hl
	call step_index
	dec de
	ld a,d
	or e
	jr nz,cp_loop
	ret

; Contract: IX is a result slot. Compares the SNAP_BYTES bytes at SNAP_LCD with those at
; LCD_BUF + PAT_LCD_BYTES, the last rows of the read-back, and fills the slot with rows counted
; from PAT_LCD_ROWS. Destroys AF BC DE HL.
cmp_snap:
	call slot_clear
	ld a,PAT_LCD_ROWS
	ld (LCD_R),a
	ld hl,SNAP_LCD
	ld de,LCD_BUF + PAT_LCD_BYTES
	ld bc,SNAP_BYTES
cs_loop:
	ld a,(de)
	cp (hl)
	call nz,note_diff
	inc hl
	inc de
	call step_index
	dec bc
	ld a,b
	or c
	jr nz,cs_loop
	ret

slot_clear:
	xor a
	ld (ix+0),a
	ld (ix+1),a
	ld (LCD_I),a
	ld (LCD_R),a
	ld (LCD_C),a
	dec a
	ld (ix+2),a
	ld (ix+3),a
	ret

; Counts one difference at the row and column in LCD_R and LCD_C, and keeps the first. Preserves
; HL, DE and BC.
note_diff:
	inc (ix+0)
	jr nz,nd_first
	inc (ix+1)
nd_first:
	ld a,(ix+2)
	inc a
	ret nz
	ld a,(LCD_R)
	ld (ix+2),a
	ld a,(LCD_C)
	ld (ix+3),a
	ret

; Moves the row and column and the pattern index on by one byte.
step_index:
	ld a,(LCD_I)
	inc a
	ld (LCD_I),a
	ld a,(LCD_C)
	inc a
	cp UI_STRIDE
	jr nz,si_column
	ld a,(LCD_R)
	inc a
	ld (LCD_R),a
	xor a
si_column:
	ld (LCD_C),a
	ret
