; fm_clip: cuts the selected statement to a number of lines with an ellipsis, as Layout::clipped
; does in crates/mcui/src/fold.rs.
;
; The model keeps the first pieces of the laid out statement and puts one ellipsis after them, the
; most pieces for which the lines still fit. The pieces before the ellipsis lie on the lines the
; statement already has, so one walk over the records finds the last visible token after which the
; ellipsis, placed as the next token, ends on a line within the limit. The record after it becomes
; the ellipsis and every later record is hidden, which leaves fm_rows and fm_draw nothing to know.
; Header byte 7 bit FM_CLIPPED marks the statement; a load, fm_fit or fm_open clear it.

; in: B = the lines to keep, D = the width in pixels
clip_body:
	call fm_need
	jp nz,fm_fail
	ld a,b
	ld (fv_kr),a
	ld a,d
	ld (fv_limit),a
	ld a,(fv_count)
	or a
	ld a,FM_ERR_INDEX
	jp z,fm_fail
	call cl_flag
	ld a,FM_ERR_CLIPPED
	jp nz,fm_fail
	call fm_count
	ld b,a
	ld a,(fv_kr)
	cp b
	jr nc,cb_fits
	ld a,$FF
	ld (fv_curn),a
	call clip_scan
	call clip_apply
	ld a,(fv_kn)
	ld b,a
cb_fits:
	xor a
	jp fm_leave

; out: NZ when the selected statement is clipped, HL = the address of its header byte 7. Destroys AF.
cl_flag:
	call fm_hdr7
	bit FM_CLIPPED,(hl)
	ret

; Walks the records of the selected statement against fv_kr lines of fv_limit pixels. in: fv_tok,
; fv_count, fv_kr, fv_limit and fv_ew, with the records fresh (fo_fresh has run). out: fv_kb = the
; record that becomes the ellipsis, fv_kp = the spacing flags of the visible token before it,
; fv_kn = the lines the cut statement takes. A hidden record has gap 0, a visible one at least 1.
; The cut at the start, an ellipsis alone, is the default and the answer when no token fits. It
; returns early with carry set, before the last record, when a token starts a line past fv_kr.
; Destroys AF BC DE HL and writes fv_kl, fv_kc, fv_kp, fv_kn and fv_kb; it writes no record.
clip_scan:
	ld hl,(fv_tok)
	ld (fv_kb),hl
	xor a
	ld (fv_kl),a
	ld (fv_kp),a
	inc a
	ld (fv_kn),a
	ld a,(fv_count)
	ld b,a
ks_next:
	inc hl
	ld c,(hl)
	inc hl
	ld a,(hl)
	inc hl
	ld d,(hl)
	inc hl
	or a
	jr z,ks_skip
	ld e,a
	ld a,(fv_kl)
	or a
	jr z,ks_line
	ld a,e
	add a,c
	jr c,ks_line
	ld e,a
	ld a,(fv_kc)
	sub e
	jr c,ks_line
	ld (fv_kc),a
	jr ks_placed
ks_line:
	ld a,(fv_kl)
	inc a
	ld (fv_kl),a
	ld e,a
	ld a,(fv_kr)
	cp e
	ret c
	ld a,(fv_limit)
	sub c
	jr nc,ks_room
	xor a
ks_room:
	ld (fv_kc),a
ks_placed:
	ld a,b
	cp 2
	jr c,ks_skip
	ld a,d
	and $0F
	ld d,a
	ld c,FM_PAD_VAR
	call fm_gap
	ld a,(fv_ew)
	add a,e
	jr c,ks_wrap
	ld c,a
	ld a,(fv_kc)
	sub c
	jr c,ks_wrap
	ld a,(fv_kl)
	jr ks_take
ks_wrap:
	ld a,(fv_kr)
	ld c,a
	ld a,(fv_kl)
	cp c
	jr nc,ks_skip
	inc a
ks_take:
	ld (fv_kn),a
	ld a,d
	ld (fv_kp),a
	ld (fv_kb),hl
ks_skip:
	djnz ks_next
	ret

; Rewrites the records from the cut on and sets the header: the line count kept for the width, the
; counted bit and the clipped bit. Destroys all but IX and IY.
clip_apply:
	ld a,(fv_kp)
	ld d,a
	ld c,FM_PAD_VAR
	call fm_gap
	ld hl,(fv_kb)
	inc hl
	ld a,(fv_ew)
	ld (hl),a
	inc hl
	ld (hl),e
	inc hl
	ld (hl),FM_PAD_VAR | (1<<FM_ELLIPSIS)
	inc hl
	push hl
	ld a,(fv_count)
	ld l,a
	ld h,0
	add hl,hl
	add hl,hl
	ld de,(fv_tok)
	add hl,de
	ex de,hl
	pop hl
ca_hide:
	ld a,l
	cp e
	jr nz,ca_one
	ld a,h
	cp d
	jr z,ca_done
ca_one:
	ld c,(hl)
	ld b,FM_SYMF>>8
	ld a,(bc)
	and $0F
	inc hl
	ld (hl),0
	inc hl
	ld (hl),0
	inc hl
	ld (hl),a
	inc hl
	jr ca_hide
ca_done:
	call ct_slot
	ld a,(fv_kn)
	ld (hl),a
	ld de,FM_MAX_STMTS
	add hl,de
	ld a,(fv_limit)
	ld (hl),a
	call fm_hdr7
	set FM_COUNTED,(hl)
	set FM_CLIPPED,(hl)
	ret
