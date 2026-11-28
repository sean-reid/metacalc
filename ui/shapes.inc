; Shapes: rectangle inversion.

; ui_invert   in:  B = x, C = y, D = width, E = height, in pixels
;             flips the rectangle, clipped to the screen, and marks the text rows its clipped
;             scanlines are in; nothing is flipped or marked when width or height is 0 or the
;             rectangle starts off the screen. Each
;             scanline is flipped a byte at a time, the bytes between the edge ones with cpl and
;             the edge ones through a mask. A rectangle that starts in the first byte column and
;             ends in the last is one run of bytes from its first scanline to its last
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
	ld a,c
	add a,e
	dec a
	rrca
	and $1C
	or (ut_rows+2)&$FF
	ld l,a
	ld h,ut_rows>>8
	ld a,(hl)
	add a,a
	ld h,a
	ld a,c
	rrca
	and $1C
	or (ut_rows+2)&$FF
	ld l,a
	ld a,h
	ld h,ut_rows>>8
	sub (hl)
	ld hl,ui_dirty
	or (hl)
	ld (hl),a
	ld a,b
	add a,d
	dec a
	ld d,a
	ld a,b
	cp 8
	jr nc,iv_gen
	ld a,d
	cp 8*(UI_STRIDE-1)
	jr c,iv_gen
	and 7
	or iv_rmasks & $FF
	ld l,a
	ld h,iv_rmasks >> 8
	ld d,(hl)
	ld a,b
	or iv_lmasks & $FF
	ld l,a
	ld b,e
	ld e,(hl)
	ld a,c
	add a,a
	add a,c
	ld l,a
	ld h,0
	add hl,hl
	add hl,hl
	ld a,l
	add a,UI_FRAME & $FF
	ld l,a
	ld a,h
	adc a,UI_FRAME >> 8
	ld h,a
	ld c,e
	ld a,b
	cp 8
	jp z,iv_rows8
	jp iv_flip_row
iv_gen:
	ld a,d
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

; The rectangle spans all twelve bytes of each scanline, so its scanlines are one run of bytes. HL =
; the first byte, B = scanlines, C = the left mask, D = the right mask.
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

; The same for eight scanlines, with no loop.
#macro ROW12
	ld a,(hl)
	xor c
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
	xor d
	ld (hl),a
	inc hl
#endmacro
; With the run inside one 256 byte page the pointer steps through L alone.
#macro ROW12L
	ld a,(hl)
	xor c
	ld (hl),a
	inc l
	ld a,(hl)
	cpl
	ld (hl),a
	inc l
	ld a,(hl)
	cpl
	ld (hl),a
	inc l
	ld a,(hl)
	cpl
	ld (hl),a
	inc l
	ld a,(hl)
	cpl
	ld (hl),a
	inc l
	ld a,(hl)
	cpl
	ld (hl),a
	inc l
	ld a,(hl)
	cpl
	ld (hl),a
	inc l
	ld a,(hl)
	cpl
	ld (hl),a
	inc l
	ld a,(hl)
	cpl
	ld (hl),a
	inc l
	ld a,(hl)
	cpl
	ld (hl),a
	inc l
	ld a,(hl)
	cpl
	ld (hl),a
	inc l
	ld a,(hl)
	xor d
	ld (hl),a
	inc l
#endmacro
iv_rows8:
	ld a,l
	add a,8*UI_STRIDE-1
	jp c,iv_rows8w
	ROW12L
	ROW12L
	ROW12L
	ROW12L
	ROW12L
	ROW12L
	ROW12L
	ROW12L
	ret
iv_rows8w:
	ROW12
	ROW12
	ROW12
	ROW12
	ROW12
	ROW12
	ROW12
	ROW12
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
