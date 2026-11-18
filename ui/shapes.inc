; Shapes: clear, rectangles, rows and the margin marker.

; ui_clear   zeroes the frame with the stack, interrupts off, and restores the interrupt state
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
	ld b,UI_FRAME_SIZE/8
uc_loop:
	push hl
	push hl
	push hl
	push hl
	djnz uc_loop
	ld sp,(ui_sp)
	ld a,(ui_iff)
	or a
	ret z
	ei
	ret

; ui_row_addr   in:  A = text row 0 to 7
;               out: HL = frame address of the first byte of the row's first glyph scanline
;               destroys AF DE
ui_row_addr:
	ld l,a
	add a,a
	add a,l
	ld l,a
	ld h,0
	add hl,hl
	add hl,hl
	add hl,hl
	add hl,hl
	add hl,hl
	ld de,UI_FRAME+UI_STRIDE
	add hl,de
	ret

; ui_marker   in:  A = text row 0 to 7
;             sets columns 0 and 1 on the seven scanlines below the row's blank one
;             destroys AF BC DE HL
ui_marker:
	call ui_row_addr
	ld bc,UI_STRIDE
	ld a,7
mk_loop:
	set 7,(hl)
	set 6,(hl)
	add hl,bc
	dec a
	jr nz,mk_loop
	ret

; ui_invert_row   in:  A = text row 0 to 7
;                 flips the eight scanlines of the row
;                 destroys AF BC DE HL
ui_invert_row:
	call ui_row_addr
	ld de,-UI_STRIDE
	add hl,de
	ld b,UI_ROW*UI_STRIDE/4
ir_loop:
	ld a,(hl)
	cpl
	ld (hl),a
	inc hl
	ld a,(hl)
	cpl
	ld (hl),a
	inc hl
	ld a,(hl)
	cpl
	ld (hl),a
	inc hl
	ld a,(hl)
	cpl
	ld (hl),a
	inc hl
	djnz ir_loop
	ret

; ui_invert   in:  B = x, C = y, D = width, E = height, in pixels
;             flips the rectangle, clipped to the screen; nothing when width or height is 0
;             destroys AF BC DE HL
ui_invert:
	ld a,d
	or a
	ret z
	ld a,e
	or a
	ret z
	ld a,b
	cp UI_WIDTH
	ret nc
	ld a,c
	cp UI_HEIGHT
	ret nc
	ld a,UI_WIDTH
	sub b
	cp d
	jr nc,iv_w
	ld d,a
iv_w:
	ld a,UI_HEIGHT
	sub c
	cp e
	jr nc,iv_h
	ld e,a
iv_h:
	ld a,e
	ld (ui_rows),a
	ld a,b
	add a,d
	dec a
	ld d,a
	ld a,c
	add a,a
	add a,c
	ld l,a
	ld h,0
	add hl,hl
	add hl,hl
	push de
	ld de,UI_FRAME
	add hl,de
	ld a,b
	rrca
	rrca
	rrca
	and $1F
	ld e,a
	ld d,0
	add hl,de
	ld (ui_dst),hl
	pop de
	ld a,b
	and 7
	ld hl,iv_lmasks
	call ui_add_hl_a
	ld a,(hl)
	ld (ui_lm),a
	ld a,d
	and 7
	ld hl,iv_rmasks
	call ui_add_hl_a
	ld a,(hl)
	ld (ui_rm),a
	ld a,b
	rrca
	rrca
	rrca
	and $1F
	ld c,a
	ld a,d
	rrca
	rrca
	rrca
	and $1F
	sub c
	jr nz,iv_wide
	ld a,(ui_lm)
	ld hl,ui_rm
	and (hl)
	ld (ui_lm),a
	ld a,$FF
	jr iv_set
iv_wide:
	dec a
iv_set:
	ld (ui_mid),a
iv_row:
	ld hl,(ui_dst)
	ld a,(ui_mid)
	inc a
	jr z,iv_single
	ld a,(ui_lm)
	xor (hl)
	ld (hl),a
	inc hl
	ld a,(ui_mid)
	or a
	jr z,iv_last
	ld b,a
iv_mid:
	ld a,(hl)
	cpl
	ld (hl),a
	inc hl
	djnz iv_mid
iv_last:
	ld a,(ui_rm)
	jr iv_flip
iv_single:
	ld a,(ui_lm)
iv_flip:
	xor (hl)
	ld (hl),a
	ld hl,(ui_dst)
	ld de,UI_STRIDE
	add hl,de
	ld (ui_dst),hl
	ld hl,ui_rows
	dec (hl)
	jr nz,iv_row
	ret

; HL = HL + A
ui_add_hl_a:
	add a,l
	ld l,a
	ret nc
	inc h
	ret

iv_lmasks:
	.db $FF,$7F,$3F,$1F,$0F,$07,$03,$01
iv_rmasks:
	.db $80,$C0,$E0,$F0,$F8,$FC,$FE,$FF
