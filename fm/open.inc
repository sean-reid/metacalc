; fm_open: copies the library's glyph section into FM_GLYPHS and checks every entry of it, then
; copies the shapes section into FM_SECOND and checks every item of it.

open_body:
	xor a
	ld (fv_open),a
	ld (fv_open+1),a
	call fm_clear
	xor a
	ld (fv_sk+5),a
	ld (fv_ns),a
	ld (fv_ns+1),a
	call lib_glyphs
	jr z,op_got
	call fm_reader_status
	jp fm_fail
op_got:
	ld (fv_lib),hl
	ld (fv_src),hl
	ld a,e
	ld (fv_src+2),a
	ld (fv_lib+2),a
	ld (fv_glen),bc
	ld (fv_lib+3),bc
	ld a,(lr_page)
	ld (fv_lib+5),a
	ld hl,FM_GLYPHS_SIZE
	or a
	sbc hl,bc
	ld a,FM_ERR_TOO_BIG
	jp c,fm_fail
	ld a,b
	or c
	ld a,FM_ERR_BAD
	jp z,fm_fail
	ld (fv_left),bc
	ld hl,FM_GLYPHS
	ld (fv_dst),hl
	call fm_copy
	jr op_check

; Copies fv_left bytes of the library from the offset in fv_src to fv_dst, a chunk at a time
; through FM_SCRATCH.
fm_copy:
	ld bc,(fv_left)
	ld a,b
	or c
	ret z
	ld a,b
	or a
	jr z,op_read
	ld bc,256
op_read:
	push bc
	ld de,FM_SCRATCH
	ld hl,(fv_src)
	ld a,(fv_src+2)
	call lib_read
	pop bc
	jr z,op_copy
	call fm_reader_status
	jp fm_fail
op_copy:
	ld hl,FM_SCRATCH
	ld de,(fv_dst)
	push bc
	ldir
	ld (fv_dst),de
	pop bc
	ld hl,(fv_src)
	ld a,(fv_src+2)
	add hl,bc
	adc a,0
	ld (fv_src),hl
	ld (fv_src+2),a
	ld hl,(fv_left)
	or a
	sbc hl,bc
	ld (fv_left),hl
	jr fm_copy

; Every symbol's offset is inside the copy and its entry, flags, count and codes, ends inside it,
; with no flag above bit 3 and at least one code: the font gives every token a glyph.
op_check:
	ld a,(FM_GLYPHS+2)
	ld (fv_nsyms),a
	ld l,a
	ld h,0
	add hl,hl
	inc hl
	inc hl
	inc hl
	ld de,(fv_glen)
	or a
	sbc hl,de
	jr z,op_table
	jr nc,op_bad
op_table:
	ld a,(fv_nsyms)
	or a
	jp z,op_shapes
	ld hl,FM_SYMF
	ld de,FM_SYMF+1
	ld (hl),0
	ld b,0
	ld c,a
	dec c
	jr z,op_one
	ldir
op_one:
	ld a,(fv_nsyms)
	ld b,a
	ld hl,FM_GLYPHS+3
op_entry:
	ld e,(hl)
	inc hl
	ld d,(hl)
	inc hl
	push hl
	push bc
	call op_entry_ok
	pop bc
	pop hl
	jr nz,op_bad
	djnz op_entry
	jp op_shapes
op_done:
	ld hl,FM_OPEN_MAGIC
	ld (fv_open),hl
	xor a
	jp fm_fail
op_bad:
	ld a,FM_ERR_BAD
	jp fm_fail

; in: DE = an entry's offset in the section. out: Z when the whole entry lies inside the copy and
; its flags are below 16. Destroys AF BC DE HL.
op_entry_ok:
	ld hl,FM_GLYPHS
	add hl,de
	jr c,oe_bad
	push hl
	ld hl,(fv_glen)
	ld de,FM_GLYPHS
	add hl,de
	ex de,hl
	pop hl
	ld bc,2
	add hl,bc
	jr c,oe_bad
	push hl
	or a
	sbc hl,de
	pop hl
	jr c,oe_head
	jr nz,oe_bad
oe_head:
	dec hl
	ld c,(hl)
	ld a,c
	or a
	jr z,oe_bad
	dec hl
	ld a,(hl)
	and $F0
	jr nz,oe_bad
	inc hl
	inc hl
	ld b,0
	add hl,bc
	or a
	sbc hl,de
	jr c,oe_ok
	jr nz,oe_bad
oe_ok:
	xor a
	ret
oe_bad:
	ld a,1
	or a
	ret
