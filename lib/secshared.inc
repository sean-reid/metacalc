; The records-end walk, the section headers and the checks every section accessor shares.

; The end of the records: the later of the directory's end and the end of the last record. A
; section may not start before it. The records are walked only as far as the last one, whose
; place the directory gives; a library whose directory lists the records out of order can end
; later than this finds.
; Every position here is below 2^22, since lib_open bounds the length by the flash pages and every
; count read adds at most 2 * 65535, so no sum carries out of 24 bits.
; out: Z and lr_hdr = the end, or NZ and A = status. Destroys all but IX, IY.
lsx_records_end:
	ld hl,(lr_count)
	xor a
	add hl,hl
	rla
	add hl,hl
	rla
	ld de,lr_dir
	call lsx_add3
	ld (lr_hdr),hl
	ld (lr_hdr+2),a
	ld hl,(lr_count)
	ld a,h
	or l
	ret z
	dec hl
	call lr_record
	jp nz,lr_bad
	ld a,(lr_dclen)
	ld c,a
	ld b,0
	ld hl,(lr_dconc)
	ld a,(lr_dconc+2)
	call lr_adv
	ld b,a
	ld a,(lr_dkind)
	or a
	ld a,b
	jr z,lsx_later
	ld b,2
	call lsx_skip
	ret nz
	ld b,2
	call lsx_skip
	ret nz
	ld b,1
	call lsx_skip
	ret nz
lsx_later:
	ld (lr_ent),hl
	ld (lr_ent+2),a
	ld hl,(lr_length)
	ld a,(lr_length+2)
	ld de,lr_ent
	call lr_cmp24
	jp c,lr_bad
	ld hl,(lr_ent)
	ld a,(lr_ent+2)
	ld de,lr_hdr
	call lr_cmp24
	jr c,lsx_later_done
	ld hl,(lr_ent)
	ld (lr_hdr),hl
	ld a,(lr_ent+2)
	ld (lr_hdr+2),a
lsx_later_done:
	xor a
	ret

; in: A:HL = offset of a 16-bit count, B = bytes per element, 1 or 2.
; out: Z and A:HL = the offset past the count and its elements, or NZ and A = status.
; Destroys all but IX, IY.
lsx_skip:
	ld c,a
	ld a,b
	ld (lr_len),a
	ld a,c
	call lw_seek
	ld a,2
	call lw_need
	ret nz
	call lw_ptr
	ld e,(hl)
	inc hl
	ld d,(hl)
	ld (lr_cn),de
	ld bc,2
	call lw_skip
	call lw_pos
	ld de,(lr_cn)
	add hl,de
	adc a,0
	ld c,a
	ld a,(lr_len)
	cp 2
	ld a,c
	jr nz,lsx_skipped
	add hl,de
	adc a,0
lsx_skipped:
	cp a
	ret

; Reads each known section's header, checks that its index fits its row, and decides whether the
; glyph map matches this build's font.
; out: Z, or NZ and A = LIB_ERR_SECTION. Destroys all but IX, IY.
lsx_headers:
	ld a,(ls_have)
	and LS_TITLES
	jr z,lsx_glyph
	ld hl,ls_ti
	ld c,2
	call lsx_head
	ret nz
	ld hl,(lr_hdr)
	ld (ls_ti_n),hl
	xor a
	add hl,hl
	rla
	add hl,hl
	rla
	ld bc,2
	call lr_adv
	ld de,ls_ti
	call lsx_first
	ret nz
lsx_glyph:
	ld a,(ls_have)
	and LS_GLYPHS
	jr z,lsx_trees
	ld hl,ls_gl
	ld c,3
	call lsx_head
	ret nz
	ld hl,(lr_hdr)
	ld (ls_gl_fp),hl
	ld a,(lr_hdr+2)
	ld l,a
	ld h,0
	add hl,hl
	ld bc,3
	add hl,bc
	xor a
	ld de,ls_gl
	call lsx_first
	ret nz
	ld a,(lr_hdr+2)
	ld hl,lr_nsyms
	cp (hl)
	jr nz,lsx_trees
	ld hl,(ls_gl_fp)
	ld de,LIB_FONT_FINGERPRINT
	or a
	sbc hl,de
	jr nz,lsx_trees
	ld hl,ls_have
	ld a,(hl)
	or LS_FONT
	ld (hl),a
lsx_trees:
	ld a,(ls_have)
	and LS_TREES
	jr z,lsx_shapes
	ld hl,ls_tr
	ld c,2
	call lsx_head
	ret nz
	ld hl,(lr_hdr)
	ld (ls_tr_n),hl
	ld de,384
	or a
	sbc hl,de
	jp nc,lsx_bad_section
	ld hl,(ls_tr_n)
	ld d,h
	ld e,l
	add hl,hl
	add hl,de
	push hl
	ld hl,(lr_count)
	xor a
	add hl,hl
	rla
	add hl,hl
	rla
	pop de
	add hl,de
	adc a,0
	ld bc,2
	call lr_adv
	ld de,ls_tr
	call lsx_first
	ret nz
lsx_shapes:
	ld a,(ls_have)
	ld b,a
	and LS_TREES
	jr z,lsx_shapes_alone
	ld a,b
	and LS_SHAPES
	jp z,lsx_bad_section
	ld hl,ls_sh
	ld c,2
	call lsx_head
	ret nz
	ld hl,(lr_hdr)
	ld de,(ls_tr_n)
	or a
	sbc hl,de
	jp nz,lsx_bad_section
	ld hl,(ls_tr_n)
	add hl,hl
	ld bc,4
	add hl,bc
	ld de,(ls_sh+3)
	or a
	sbc hl,de
	jp z,lsx_kinds
	jp nc,lsx_bad_section
	jr lsx_kinds
lsx_shapes_alone:
	ld a,b
	and LS_SHAPES
	jr z,lsx_kinds
	jp lsx_bad_section
lsx_kinds:
	ld a,(ls_have)
	and LS_KINDS
	jr z,lsx_order
	ld hl,ls_kd
	ld c,2
	call lsx_head
	ret nz
	ld hl,(lr_hdr)
	ld de,(lr_count)
	or a
	sbc hl,de
	jp nz,lsx_bad_section
lsx_order:
	ld a,(ls_have)
	and LS_ORDER
	ret z
	ld hl,ls_od
	ld c,2
	call lsx_head
	ret nz
	ld hl,(lr_hdr)
	ld de,(lr_count)
	or a
	sbc hl,de
	jp nz,lsx_bad_section
	ret

; in: A:HL = the bytes between a section's start and its first item, DE = the section's
; struct. Sets the struct's first item and checks it is not past the section's end.
; out: Z, or NZ and A = LIB_ERR_SECTION. Destroys all but IX, IY.
lsx_first:
	push de
	call lsx_add3
	pop de
	jp c,lsx_bad_section
	ld (ls_te),hl
	ld (ls_te+2),a
	ex de,hl
	push hl
	ld de,LS_FIRST
	add hl,de
	ex de,hl
	ld hl,ls_te
	ld bc,3
	ldir
	pop hl

; in: HL = a section's struct. out: Z when its first item is not past its end, else NZ and
; A = LIB_ERR_SECTION. Destroys all but IX, IY.
lsx_fits:
	push hl
	call lsx_ld3
	pop de
	push de
	inc de
	inc de
	inc de
	call lsx_add3
	ld (ls_te),hl
	ld (ls_te+2),a
	pop hl
	ld de,LS_FIRST
	add hl,de
	ex de,hl
	ld hl,(ls_te)
	ld a,(ls_te+2)
	call lr_cmp24
	jp c,lsx_bad_section
	xor a
	ret

; in: HL = a section's struct, C = its header's size, 2 or 3. Reads the header into lr_hdr.
; out: Z, or NZ and A = the read's status. Destroys all but IX, IY.
lsx_head:
	ld b,0
	ld (lr_cn),bc
	call lsx_ld3
	ld bc,(lr_cn)
	ld de,lr_hdr
	jp lr_rd

; in: A:HL = the start of an item, BC = the bytes it needs, DE = a section's struct. out: Z when
; the item lies from the section's first item to its end, else NZ and A = LIB_ERR_BAD.
; Destroys all but IX, IY.
lsx_within:
	ld (ls_ts),hl
	ld (ls_ts+2),a
	call lr_adv
	jp c,lr_bad
	ld (ls_te),hl
	ld (ls_te+2),a
	push de
	ld hl,LS_FIRST
	add hl,de
	ex de,hl
	ld hl,(ls_ts)
	ld a,(ls_ts+2)
	call lr_cmp24
	pop hl
	jp c,lr_bad

; in: HL = a section's struct, ls_te = the end of an item. out: Z when the item ends inside the
; section, else NZ and A = LIB_ERR_BAD. Destroys all but IX, IY.
lsx_item_ends:
	push hl
	call lsx_ld3
	pop de
	inc de
	inc de
	inc de
	call lsx_add3
	ld de,ls_te
	call lr_cmp24
	jp c,lr_bad
	xor a
	ret

; in: as lsx_within, BC at least 1. out: Z when the item also sits inside one 16 KB chunk, else NZ
; and A = LIB_ERR_BAD. Destroys all but IX, IY.
lsx_item:
	call lsx_within
	ret nz

; in: ls_ts = the start of an item, ls_te = its end, past at least one byte. out: Z when its bytes
; sit inside one 16 KB chunk, else NZ and A = LIB_ERR_BAD. Destroys AF BC HL.
lsx_chunk:
	ld hl,(ls_te)
	ld a,(ls_te+2)
	ld bc,1
	or a
	sbc hl,bc
	sbc a,0
	ld b,a
	ld a,(ls_ts+2)
	cp b
	jp nz,lr_bad
	ld a,(ls_ts+1)
	xor h
	and $C0
	jp nz,lr_bad
	xor a
	ret

; out: NZ and A = LIB_ERR_NONE, for a library without the section asked for.
lsx_nothing:
	ld a,LIB_ERR_NONE
	or a
	ret

; in: A:HL = the position of a u32 index entry. out: Z and A:HL = its value, or NZ and A = status
; when the read fails or the value passes 24 bits. Destroys all but IX, IY.
lsx_entry4:
	ld de,lr_hdr
	ld bc,4
	call lr_rd
	ret nz
	ld a,(lr_hdr+3)
	or a
	jp nz,lr_bad
	ld hl,(lr_hdr)
	ld a,(lr_hdr+2)
	cp a
	ret
