; Shapes: clear, rectangles, rows and the margin marker.

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

; ui_marker   in:  A = text row 0 to 7, nothing drawn for a larger one
;             sets columns 0 and 1 on the seven scanlines below the row's blank one
;             destroys AF BC DE HL
ui_marker:
	cp UI_TEXT_ROWS
	ret nc
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

; ui_invert_row   in:  A = text row 0 to 7, nothing drawn for a larger one
;                 flips the seven scanlines of the row below its top one
;                 destroys AF BC DE HL
ui_invert_row:
	cp UI_TEXT_ROWS
	ret nc
	call ui_row_addr
	ld b,5
ir_loop:
#macro CPL4
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
#endmacro
	CPL4
	CPL4
	CPL4
	CPL4
	djnz ir_loop
	CPL4
	ret

; ui_invert   in:  B = x, C = y, D = width, E = height, in pixels
;             flips the rectangle, clipped to the screen; nothing when width or height is 0. Each
;             scanline is flipped a byte at a time, the bytes between the edge ones with cpl and
;             the edge ones through a mask. A rectangle that reaches both sides of the screen is
;             one run of bytes from its first scanline to its last
;             destroys AF BC DE HL
ui_invert:
	ld a,d
	or a
	ret z
	ld a,e
	or a
	ret z
	ld a,UI_WIDTH
	sub b
	ret z
	ret c
	cp d
	jr nc,iv_w
	ld d,a
iv_w:
	ld a,UI_HEIGHT
	sub c
	ret z
	ret c
	cp e
	jr nc,iv_h
	ld e,a
iv_h:
	ld a,b
	add a,d
	dec a
	ld d,a
	and 7
	or iv_rmasks & $FF
	ld l,a
	ld h,iv_rmasks >> 8
	ld a,(hl)
	ld (ui_rm),a
	ld a,b
	and 7
	or iv_lmasks & $FF
	ld l,a
	ld a,(hl)
	ld (ui_lm),a
	ld a,d
	rrca
	rrca
	rrca
	and $1F
	ld d,a
	ld a,b
	rrca
	rrca
	rrca
	and $1F
	ld b,a
	ld a,d
	sub b
	ld d,a
	ld a,c
	add a,a
	add a,c
	ld l,a
	ld h,0
	add hl,hl
	add hl,hl
	ld a,b
	add a,l
	ld l,a
	jr nc,iv_a
	inc h
iv_a:
	ld bc,UI_FRAME
	add hl,bc
	ld a,(ui_lm)
	ld c,a
	ld b,e
	ld a,d
	or a
	jp z,iv_one
	cp UI_STRIDE-1
	jp z,iv_run
	ld a,UI_STRIDE
	sub d
	ld (ui_mid),a
	ld a,d
	dec a
	add a,a
	add a,a
	ex de,hl
	push de
	ld e,a
	ld d,0
	ld hl,iv_chain_end
	or a
	sbc hl,de
	ld (ui_dst),hl
	pop de
iv_row:
	ld a,(de)
	xor c
	ld (de),a
	inc de
	ld hl,(ui_dst)
	jp (hl)
#macro CPL_DE
	ld a,(de)
	cpl
	ld (de),a
	inc de
#endmacro
iv_chain:
	CPL_DE
	CPL_DE
	CPL_DE
	CPL_DE
	CPL_DE
	CPL_DE
	CPL_DE
	CPL_DE
	CPL_DE
	CPL_DE
iv_chain_end:
	ld hl,ui_rm
	ld a,(de)
	xor (hl)
	ld (de),a
	ld a,(ui_mid)
	add a,e
	ld e,a
	jr nc,iv_next
	inc d
iv_next:
	djnz iv_row
	ret

; The rectangle spans all twelve bytes of each scanline, so its scanlines are one run of bytes.
iv_run:
	ld a,(ui_rm)
	ld d,a
#macro CPL_HL
	ld a,(hl)
	cpl
	ld (hl),a
	inc hl
#endmacro
iv_flip_row:
	ld a,(hl)
	xor c
	ld (hl),a
	inc hl
	CPL_HL
	CPL_HL
	CPL_HL
	CPL_HL
	CPL_HL
	CPL_HL
	CPL_HL
	CPL_HL
	CPL_HL
	CPL_HL
	ld a,(hl)
	xor d
	ld (hl),a
	inc hl
	djnz iv_flip_row
	ret

; One byte column: HL = its first byte, B = scanlines, C = the left mask.
iv_one:
	ld a,(ui_rm)
	and c
	ld c,a
	ld de,UI_STRIDE
iv_flip:
	ld a,(hl)
	xor c
	ld (hl),a
	add hl,de
	djnz iv_flip
	ret

; HL = HL + A
ui_add_hl_a:
	add a,l
	ld l,a
	ret nc
	inc h
	ret

	.fill (16-($&15))&15,0
iv_lmasks:
	.db $FF,$7F,$3F,$1F,$0F,$07,$03,$01
iv_rmasks:
	.db $80,$C0,$E0,$F0,$F8,$FC,$FE,$FF
