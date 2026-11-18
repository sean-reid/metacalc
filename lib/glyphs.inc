; The glyph map. See lib.z80 for the contract.

; out: Z when the library has a glyph map for this build's font, else NZ and A = LIB_ERR_NONE
; for no map or LIB_ERR_GLYPHS for another font's. Destroys AF.
lgl_state:
	ld a,(ls_have)
	and LS_GLYPHS
	jp z,lsx_nothing
	ld a,(ls_have)
	and LS_FONT
	jr z,lgl_foreign
	xor a
	ret
lgl_foreign:
	ld a,LIB_ERR_GLYPHS
	or a
	ret

lib_glyph_map:
	call lr_need
	ret nz
	ld hl,(ls_gl_fp)
	jr lgl_state

lib_glyph:
	ld (lr_bufp),de
	ld (lr_id),a
	call lr_need
	ret nz
	ld bc,255
	call lr_chkbuf
	ret nz
	call lgl_state
	ret nz
	ld a,(lr_nsyms)
	ld hl,lr_id
	cp (hl)
	jp z,lr_bad_index
	jp c,lr_bad_index
	ld l,(hl)
	ld h,0
	add hl,hl
	ld bc,3
	add hl,bc
	xor a
	ld de,ls_gl
	call lsx_add3
	ld de,lr_hdr
	ld bc,2
	call lr_rd
	ret nz
	ld hl,(lr_hdr)
	xor a
	ld de,ls_gl
	call lsx_add3
	jp c,lr_bad
	ld (lr_tr),hl
	ld (lr_tr+2),a
	ld de,lr_hdr
	ld bc,2
	call lr_rd
	ret nz
	ld a,(lr_hdr)
	and $F0
	jp nz,lr_bad
	ld a,(lr_hdr+1)
	ld c,a
	ld b,0
	inc bc
	inc bc
	ld hl,(lr_tr)
	ld a,(lr_tr+2)
	ld de,ls_gl
	call lsx_within
	ret nz
	ld a,(lr_hdr+1)
	or a
	jr z,lgl_done
	ld c,a
	ld b,0
	push bc
	ld bc,2
	ld hl,(lr_tr)
	ld a,(lr_tr+2)
	call lr_adv
	pop bc
	ld de,(lr_bufp)
	call lr_rd
	ret nz
lgl_done:
	ld a,(lr_hdr+1)
	ld b,a
	ld a,(lr_hdr)
	ld c,a
	xor a
	ret
