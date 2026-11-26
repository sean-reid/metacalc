#ifndef LIB_FONT_FINGERPRINT
#if UI_FONT_FINGERPRINT >= 0
#else
 .error "UI_FONT_FINGERPRINT must be defined before lib/lib.z80: include the font first"
#endif
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
	jr z,lsx_keep
	cp 4
	jr z,lsx_keep_shapes
	cp 5
	jr z,lsx_keep_kinds
	cp 6
	jr z,lsx_keep_order
	jp lsx_ends
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
	jp lsx_ends
lsx_keep_shapes:
	ld hl,ls_have
	ld a,(hl)
	or LS_SHAPES
	ld (hl),a
	ld hl,(lr_bufp)
	inc hl
	ld de,ls_sh
	ld bc,3
	ldir
	inc hl
	ld bc,2
	ldir
	ld a,(hl)
	or a
	jp nz,lsx_bad_section
	jp lsx_ends
lsx_keep_kinds:
	ld hl,ls_have
	ld a,(hl)
	or LS_KINDS
	ld (hl),a
	ld hl,(lr_bufp)
	inc hl
	ld de,ls_kd
	ld bc,3
	ldir
	inc hl
	ld e,(hl)
	inc hl
	ld d,(hl)
	inc hl
	ld a,(hl)
	inc hl
	or (hl)
	jp nz,lsx_bad_section
	ld hl,(lr_count)
	ld bc,7
	add hl,bc
	rr h
	rr l
	srl h
	rr l
	srl h
	rr l
	inc hl
	inc hl
	or a
	sbc hl,de
	jp nz,lsx_bad_section
	jp lsx_ends
lsx_keep_order:
	ld hl,ls_have
	ld a,(hl)
	or LS_ORDER
	ld (hl),a
	ld hl,(lr_bufp)
	inc hl
	ld de,ls_od
	ld bc,3
	ldir
	inc hl
	ld e,(hl)
	inc hl
	ld d,(hl)
	inc hl
	ld c,(hl)
	inc hl
	ld a,(hl)
	or a
	jp nz,lsx_bad_section
	ld hl,(lr_count)
	xor a
	add hl,hl
	rla
	push de
	ld de,2
	add hl,de
	adc a,0
	pop de
	cp c
	jp nz,lsx_bad_section
	or a
	sbc hl,de
	jp nz,lsx_bad_section
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
