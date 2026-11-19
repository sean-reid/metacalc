; fm_statement: loads one statement's symbols and each token's width and spacing flags.

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

statement_body:
	call fm_need
	jp nz,fm_fail
	ld a,e
	ld (fv_k),a
	ld (fv_cn),hl
	xor a
	ld (fv_count),a
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
	ld a,b
	ld (fv_j),a
	or a
	jr z,sb_done
	ld c,b
	ld b,0
	ld hl,FM_SCRATCH
	ld de,FM_IDS
	ldir
	xor a
	ld (fv_i),a
	ld (fv_prev),a
sb_token:
	ld a,(fv_i)
	ld hl,fv_j
	cp (hl)
	jr z,sb_done
	ld h,FM_IDS>>8
	ld l,a
	ld a,(hl)
	ld hl,fv_nsyms
	cp (hl)
	jr nc,sb_bad
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
	jr nz,sb_bad
	ld a,(fv_i)
	ld l,a
	ld h,FM_WIDTHS>>8
	ld (hl),e
	inc h
	push hl
	ld a,(fv_tmp)
	ld c,a
	ld a,(fv_prev)
	ld d,a
	ld a,c
	ld (fv_prev),a
	call fm_gap
	pop hl
	ld (hl),e
	inc h
	ld (hl),c
	ld hl,fv_i
	inc (hl)
	jr sb_token
sb_done:
	ld a,(fv_j)
	ld (fv_count),a
	ld b,a
	xor a
	jp fm_leave
sb_bad:
	ld a,FM_ERR_BAD
	jp fm_fail
