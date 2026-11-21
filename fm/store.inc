; The store: every statement of one assertion, loaded by fm_assertion, and fm_select, which makes one
; of them the statement the other routines work on.

; Drops what is loaded. Destroys AF HL.
fm_clear:
	xor a
	ld (fv_nhyps),a
	ld (fv_nw),a
	ld (fv_count),a
	ld h,a
	ld l,a
	ld (fv_ntok),hl
	ld (fv_nused),hl
	dec a
	ld (fv_last),a
	ld (fv_curn),a
	ld hl,FM_TOKENS
	ld (fv_tokp),hl
	ret

; in: A = a statement's index in the store, at most 31. out: A = 0, B = its token count. The first
; three bytes of the statement's header, its tokens and their count, are copied to fv_tok and
; fv_count, and fv_hdr points at the header. Destroys all but IX and IY.
sel_index:
	add a,a
	add a,a
	add a,a
	ld l,a
	ld h,FM_HDR>>8
	ld (fv_hdr),hl
	ld de,fv_tok
	ldi
	ldi
	ldi
	dec hl
	ld b,(hl)
	xor a
	ret

assertion_body:
	call fm_need
	jp nz,fm_fail
	ld (fv_cn),hl
	call fm_clear
	call fm_same_library
	jp nz,fm_fail
	xor a
	ld (fv_j),a
as_hyp:
	ld hl,(fv_cn)
	ld a,(fv_j)
	ld b,a
	ld de,FM_SCRATCH
	call lib_hypothesis
	jr z,as_got
	cp LIB_ERR_INDEX
	jr z,as_conclusion
	call fm_reader_status
	jp fm_fail
as_got:
	ld a,c
	inc a
	jr nz,as_next
	ld a,(fv_nw)
	ld c,a
	call ld_statement
	jp nz,fm_fail
as_next:
	ld hl,fv_j
	inc (hl)
	jr nz,as_hyp
as_conclusion:
	ld hl,(fv_cn)
	ld de,FM_SCRATCH
	call lib_conclusion
	jr z,as_last
	call fm_reader_status
	jp fm_fail
as_last:
	ld c,$FF
	call ld_statement
	jp nz,fm_fail
	ld a,(fv_nw)
	dec a
	ld (fv_last),a
	ld (fv_nhyps),a
	call sel_index
	ld a,(fv_nw)
	ld b,a
	xor a
	jp fm_leave

; in: E = the statement, as fm_statement numbers it.
select_body:
	call fm_need
	jp nz,fm_fail
	ld a,e
	inc a
	jr z,sl_conclusion
	ld a,e
	ld hl,fv_nhyps
	cp (hl)
	jr nc,sl_index
	jr sl_go
sl_conclusion:
	ld a,(fv_last)
	inc a
	jr z,sl_index
	dec a
sl_go:
	ld hl,fv_curn
	ld (hl),$FF
	call sel_index
	jp fm_leave
sl_index:
	ld a,FM_ERR_INDEX
	jp fm_fail
