#ifndef LIB_FONT_FINGERPRINT
LIB_FONT_FINGERPRINT .equ UI_FONT_FINGERPRINT
#endif

; The trailer and section table, read at the end of lib_open, and the checks every section
; accessor shares. A section's row is kept as a 9 byte struct: offset, length, first item (24-bit
; each, little-endian), at ls_ti, ls_gl and ls_tr.

lsx_magic:
	.db "MCSX"

; Reads the trailer and the table into the caller's scratch and fills the ls_ fields. A library
; without the trailer magic has no sections and passes.
; out: Z, or NZ and A = status. Destroys all but IX, IY.
lsx_open:
	xor a
	ld (ls_have),a
	ld h,a
	ld l,a
	ld (ls_ti_n),hl
	ld (ls_gl_fp),hl
	ld (ls_tr_n),hl
	ld hl,(lr_length)
	ld a,(lr_length+2)
	ld bc,6
	or a
	sbc hl,bc
	sbc a,0
	ld (lr_ent),hl
	ld (lr_ent+2),a
	ld de,(lr_scratch)
	ld bc,6
	call lr_rd
	ret nz
	ld hl,(lr_scratch)
	inc hl
	inc hl
	ld de,lsx_magic
	ld b,4
lsx_magic_loop:
	ld a,(de)
	cp (hl)
	jr nz,lsx_none
	inc de
	inc hl
	djnz lsx_magic_loop
	ld hl,(lr_scratch)
	inc hl
	ld a,(hl)
	cp 1
	ld a,LIB_ERR_TRAILER
	ret nz
	dec hl
	ld a,(hl)
	ld (lr_sid),a
	or a
	jr z,lsx_none
	cp 29
	jr nc,lsx_bad_table
	ld l,a
	ld h,0
	ld e,l
	ld d,h
	add hl,hl
	add hl,hl
	add hl,hl
	add hl,de
	ld (lr_cn),hl
	ld b,h
	ld c,l
	ld hl,(lr_ent)
	ld a,(lr_ent+2)
	or a
	sbc hl,bc
	sbc a,0
	jr c,lsx_bad_table
	ld (lr_tr),hl
	ld (lr_tr+2),a
	ld de,lo_hdr_len
	call lr_cmp24
	jr c,lsx_bad_table
	ld hl,(lr_tr)
	ld a,(lr_tr+2)
	ld bc,(lr_cn)
	ld de,(lr_scratch)
	call lr_rd
	ret nz
	call lsx_records_end
	push af
	ld hl,$FFFF
	ld (lr_dn),hl
	pop af
	ret nz
	xor a
	ld (lr_sbase),a
lsx_row:
	ld a,(lr_sbase)
	ld hl,lr_sid
	cp (hl)
	jr nc,lsx_rows_done
	call lsx_rowptr
	call lsx_check
	ret nz
	ld hl,lr_sbase
	inc (hl)
	jr lsx_row
lsx_rows_done:
	call lsx_headers
	ret nz
lsx_none:
	xor a
	ret
lsx_bad_table:
	ld a,LIB_ERR_TABLE
	or a
	ret
lsx_bad_section:
	ld a,LIB_ERR_SECTION
	or a
	ret

; in: A = row number. out: HL = the row in the scratch, 9 bytes: id, offset (4), length (4).
; Destroys AF DE.
lsx_rowptr:
	ld e,a
	add a,a
	add a,a
	add a,a
	add a,e
	ld e,a
	ld d,0
	ld hl,(lr_scratch)
	add hl,de
	ret

; in: HL = the row at index lr_sbase, lr_hdr = the end of the records, lr_tr = the table's start.
; Checks the row against them and the rows before it, and keeps its place if the id is known. The
; row's length becomes its end.
; out: Z, or NZ and A = LIB_ERR_TABLE. Destroys all but IX, IY.
lsx_check:
	ld (lr_bufp),hl
	inc hl
	ld a,(hl)
	and 3
	jp nz,lsx_bad_table
	ld de,3
	add hl,de
	ld a,(hl)
	ld de,4
	add hl,de
	or (hl)
	jp nz,lsx_bad_table
	ld hl,(lr_bufp)
	inc hl
	call lsx_ld3
	ld de,lr_hdr
	call lr_cmp24
	jp c,lsx_bad_table
	ld hl,(lr_bufp)
	ld de,5
	add hl,de
	ex de,hl
	ld hl,(lr_bufp)
	inc hl
	call lsx_ld3
	call lsx_add3
	jp c,lsx_bad_table
	ld (lr_ent),hl
	ld (lr_ent+2),a
	ld hl,(lr_tr)
	ld a,(lr_tr+2)
	ld de,lr_ent
	call lr_cmp24
	jp c,lsx_bad_table
	ld hl,(lr_bufp)
	ld a,(hl)
	ld de,ls_ti
	ld b,LS_TITLES
	cp 1
	jr z,lsx_keep
	ld de,ls_gl
	ld b,LS_GLYPHS
	cp 2
	jr z,lsx_keep
	ld de,ls_tr
	ld b,LS_TREES
	cp 3
	jr nz,lsx_ends
lsx_keep:
	ld hl,ls_have
	ld a,(hl)
	or b
	ld (hl),a
	ld hl,(lr_bufp)
	inc hl
	ld bc,3
	ldir
	inc hl
	ld bc,3
	ldir
lsx_ends:
	ld hl,(lr_bufp)
	ld de,5
	add hl,de
	ex de,hl
	ld hl,lr_ent
	ld bc,3
	ldir
	xor a
	ld (lr_want),a
lsx_pair:
	ld a,(lr_want)
	ld hl,lr_sbase
	cp (hl)
	jr nc,lsx_checked
	call lsx_rowptr
	ld (lr_q),hl
	ld a,(hl)
	ld hl,(lr_bufp)
	cp (hl)
	jp z,lsx_bad_table
	ld de,(lr_q)
	call lsx_before
	jr nc,lsx_pair_next
	ld hl,(lr_q)
	ld de,(lr_bufp)
	call lsx_before
	jp c,lsx_bad_table
lsx_pair_next:
	ld hl,lr_want
	inc (hl)
	jr lsx_pair
lsx_checked:
	xor a
	ret

; in: HL, DE = rows whose lengths have become ends. out: C if the row at HL starts before the
; row at DE ends. Destroys all but IX, IY.
lsx_before:
	inc hl
	ld a,5
	add a,e
	ld e,a
	jr nc,lsx_before_lt
	inc d
lsx_before_lt:

; in: HL, DE = addresses of 24-bit values a and b. out: C if a < b. Destroys AF BC DE HL.
lsx_lt:
	ld c,(hl)
	inc hl
	ld b,(hl)
	inc hl
	ld a,(hl)
	ld h,b
	ld l,c
	jp lr_cmp24

; in: HL = address of a 24-bit value. out: A:HL = the value. Destroys BC.
lsx_ld3:
	ld c,(hl)
	inc hl
	ld b,(hl)
	inc hl
	ld a,(hl)
	ld h,b
	ld l,c
	ret

; in: A:HL = a, DE = address of a 24-bit value b. out: A:HL = a + b, C if it passes 24 bits.
; Destroys B DE.
lsx_add3:
	ld b,a
	ld a,(de)
	add a,l
	ld l,a
	inc de
	ld a,(de)
	adc a,h
	ld h,a
	inc de
	ld a,(de)
	adc a,b
	ret

; The end of the records: the later of the directory's end and the end of the last record. A
; section may not start before it. The records are walked only as far as the last one, whose
; place the directory gives; a library whose directory lists the records out of order can end
; later than this finds.
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
	ret z
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
	jp lsx_first

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
