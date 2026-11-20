; fm_statement and the loader fm_assertion shares: a statement's symbols, each token's width and
; spacing flags, and the statement's header in the store.

; in: A = a symbol id below fv_nsyms. out: HL = the symbol's glyph codes, with a 0 written after
; them over the byte that was there; fm_restore puts that byte back. Destroys AF BC DE.
fm_codes:
	ld l,a
	ld h,0
	add hl,hl
	ld de,FM_GLYPHS+3
	add hl,de
	ld e,(hl)
	inc hl
	ld d,(hl)
	ld hl,FM_GLYPHS+1
	add hl,de
	ld c,(hl)
	inc hl
	push hl
	ld b,0
	add hl,bc
	ld a,(hl)
	ld (fv_byte),a
	ld (hl),b
	ld (fv_end),hl
	pop hl
	ret

; Destroys AF HL.
fm_restore:
	ld hl,(fv_end)
	ld a,(fv_byte)
	ld (hl),a
	ret

; in: D = the previous token's spacing flags, C = this token's. out: E = the columns between
; them, 1 and FM_SPACE more when the flags ask for it. Destroys nothing else.
fm_gap:
	ld e,1
	bit 3,d
	ret nz
	bit 1,d
	jr nz,fg_spaced
	bit 0,c
	jr nz,fg_spaced
	bit 2,d
	ret z
	bit 2,c
	ret z
fg_spaced:
	ld e,1+FM_SPACE
	ret

; out: Z when the reader has open the library fm_open copied from, else NZ and A =
; FM_ERR_NOT_OPEN: the glyph section at the same place and of the same length in the same flash
; page, the same constant and symbol counts, the same syntax axiom count, and the shapes section
; at the same place and of the same length, or none in both. Destroys all but IX, IY.
fm_same_library:
	call lib_glyphs
	jr nz,fs_other
	ld a,(fv_lib+2)
	cp e
	jr nz,fs_other
	ld de,(fv_lib)
	or a
	sbc hl,de
	jr nz,fs_other
	ld hl,(fv_lib+3)
	or a
	sbc hl,bc
	jr nz,fs_other
	ld a,(lr_page)
	ld hl,fv_lib+5
	cp (hl)
	jr nz,fs_other
	call lib_symbol_counts
	jr nz,fs_other
	ld a,(fv_consts)
	cp b
	jr nz,fs_other
	ld a,b
	add a,c
	ld hl,fv_nsyms
	cp (hl)
	jr nz,fs_other
	call lib_syntax_count
	jr z,fs_trees
	cp LIB_ERR_NONE
	jr nz,fs_other
	ld hl,0
fs_trees:
	ld de,(fv_ns)
	or a
	sbc hl,de
	jr nz,fs_other
	call lib_shapes
	jr z,fs_shapes
	cp LIB_ERR_NONE
	jr nz,fs_other
	ld a,(fv_sk+5)
	or a
	jr nz,fs_other
	ret
fs_shapes:
	ld a,(fv_sk+5)
	or a
	jr z,fs_other
	ld a,(fv_sk+2)
	cp e
	jr nz,fs_other
	ld de,(fv_sk)
	or a
	sbc hl,de
	jr nz,fs_other
	ld hl,(fv_sk+3)
	or a
	sbc hl,bc
	jr nz,fs_other
	xor a
	ret
fs_other:
	ld a,FM_ERR_NOT_OPEN
	or a
	ret

statement_body:
	call fm_need
	jp nz,fm_fail
	ld a,e
	ld (fv_k),a
	ld (fv_cn),hl
	call fm_clear
	call fm_same_library
	jp nz,fm_fail
	ld a,(fv_k)
	inc a
	jr z,sb_conclusion
	xor a
	ld (fv_j),a
	ld (fv_seen),a
sb_next:
	ld hl,(fv_cn)
	ld a,(fv_j)
	ld b,a
	ld de,FM_SCRATCH
	call lib_hypothesis
	jr z,sb_one
	call fm_reader_status
	jp fm_fail
sb_one:
	ld a,c
	inc a
	jr nz,sb_skip
	ld a,(fv_k)
	ld hl,fv_seen
	cp (hl)
	jr z,sb_load
	inc (hl)
sb_skip:
	ld hl,fv_j
	inc (hl)
	jr nz,sb_next
	ld a,FM_ERR_INDEX
	jp fm_fail
sb_conclusion:
	ld hl,(fv_cn)
	ld de,FM_SCRATCH
	call lib_conclusion
	jr z,sb_load
	call fm_reader_status
	jp fm_fail

; in: B = the symbol count, the symbols in FM_SCRATCH.
sb_load:
	ld a,(fv_k)
	ld c,a
	call ld_statement
	jp nz,fm_fail
	ld a,(fv_k)
	inc a
	ld a,1
	jr nz,sb_hyp
	dec a
	ld (fv_last),a
sb_hyp:
	ld (fv_nhyps),a
	xor a
	call sel_index
	jp fm_leave

; Appends the statement in FM_SCRATCH to the store: its header, then a record of four bytes for
; each token, the symbol id, the width, the columns before it and its spacing flags.
; in: B = the symbol count, C = the statement's number for lib_tree.
; out: Z, or NZ with A = FM_ERR_LIMIT when the store is full or FM_ERR_BAD for a symbol past the
; glyph map or a token wider than 255 pixels. Destroys all but IX and IY.
ld_statement:
	ld a,(fv_nw)
	cp FM_MAX_STMTS
	jp nc,lt_limit
	ld hl,(fv_ntok)
	ld e,b
	ld d,0
	add hl,de
	ld (fv_ntok),hl
	ld de,FM_MAX_TOKENS+1
	or a
	sbc hl,de
	jp nc,lt_limit
	ld a,(fv_nw)
	ld l,a
	ld h,0
	add hl,hl
	add hl,hl
	add hl,hl
	ld de,FM_HDR
	add hl,de
	ld de,(fv_tokp)
	ld (hl),e
	inc hl
	ld (hl),d
	inc hl
	ld (hl),b
	inc hl
	xor a
	ld (hl),a
	inc hl
	ld (hl),a
	inc hl
	ld (hl),a
	inc hl
	ld (hl),c
	inc hl
	ld (hl),0
	ld a,b
	or a
	jr z,lt_done
	ld (fv_i),a
	ld hl,FM_SCRATCH
	ld (fv_ip),hl
	xor a
	ld (fv_prev),a
lt_token:
	ld hl,(fv_ip)
	ld a,(hl)
	inc hl
	ld (fv_ip),hl
	ld hl,fv_nsyms
	cp (hl)
	jp nc,lt_bad
	ld (fv_id),a
	ld e,a
	ld d,FM_SYMF>>8
	ld a,(de)
	or a
	jp m,lt_have
	call lt_measure
	jp nz,lt_failed
	ld a,(fv_id)
	ld e,a
	ld d,FM_SYMF>>8
	ld a,(de)
lt_have:
	and $0F
	ld c,a
	dec d
	ld a,(de)
	ld b,a
	ld a,(fv_prev)
	ld d,a
	ld a,c
	ld (fv_prev),a
	call fm_gap
	ld hl,(fv_tokp)
	ld a,(fv_id)
	ld (hl),a
	inc hl
	ld (hl),b
	inc hl
	ld (hl),e
	inc hl
	ld (hl),c
	inc hl
	ld (fv_tokp),hl
	ld hl,fv_i
	dec (hl)
	jp nz,lt_token
lt_done:
	ld hl,fv_nw
	inc (hl)
	xor a
	ret
lt_bad:
	ld a,FM_ERR_BAD
lt_failed:
	or a
	ret
lt_limit:
	ld a,FM_ERR_LIMIT
	or a
	ret

; Measures symbol fv_id once for the library: its token's width and spacing flags go to FM_SYMW and
; FM_SYMF, the flags with bit 7 set. out: Z, or NZ with A = FM_ERR_BAD for a token wider than 255
; pixels. Destroys all but IX and IY.
lt_measure:
	ld a,(fv_id)
	call fm_codes
	push hl
	dec hl
	dec hl
	ld a,(hl)
	ld (fv_tmp),a
	pop hl
	call ui_text_width
	ex de,hl
	call fm_restore
	ld a,d
	or a
	jr nz,lt_unfit
	ld a,(fv_id)
	ld l,a
	ld h,FM_SYMW>>8
	ld (hl),e
	inc h
	ld a,(fv_tmp)
	or $80
	ld (hl),a
	xor a
	ret
lt_unfit:
	ld a,FM_ERR_BAD
	or a
	ret
