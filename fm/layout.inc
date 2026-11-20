; fm_rows and fm_draw: lay the loaded tokens out in lines the way mcglyph's place does, and draw the
; lines inside a window.

; in: D = width in pixels
rows_body:
	call fm_need
	jp nz,fm_fail
	ld a,d
	ld (fv_limit),a
	call fo_fresh
	xor a
	ld (fv_draw),a
	call fm_lay
	ld a,(fv_line)
	ld b,a
	xor a
	jp fm_leave

; in: HL = the text row of the first line, signed, from -16384 to 16383; D = width in pixels,
; E = x of the first column, B = the window's first text row, C = one past its last
draw_body:
	call fm_need
	jp nz,fm_fail
	ld a,d
	ld (fv_limit),a
	ld a,e
	ld (fv_x0),a
	ld (fv_row),hl
	ld a,b
	ld (fv_lo),a
	ld a,c
	ld (fv_hi),a
	call fo_fresh
	ld a,1
	ld (fv_draw),a
	call fm_lay
	xor a
	ld b,a
	jp fm_leave

; Breaks the tokens into lines and draws the ones on a visible line when fv_draw is set. A token
; goes on the current line when the line's width, its gap and its own width fit the width, else it
; starts a line. The shadow registers hold the walk through the records: HL' the next token's
; record, B' the tokens left, C' the columns left on the line. ui_text and the window code leave
; them alone.
fm_lay:
	xor a
	ld (fv_line),a
	ld (fv_show),a
	ld a,(fv_count)
	or a
	ret z
	exx
	ld b,a
	ld hl,(fv_tok)
	exx
ly_line:
	ld hl,fv_line
	inc (hl)
	ld a,(fv_draw)
	or a
	jr z,ly_skip
	call fy_window
	ret c
	ld a,(fv_show)
	or a
	jr z,ly_skip
	call ly_draw
	jr ly_more
ly_skip:
	call ly_scan
ly_more:
	exx
	ld a,b
	exx
	or a
	jr nz,ly_line
	ret

; Moves HL' and B' past the next line's tokens. A first token wider than the width leaves no
; column, and every later token needs at least its gap of 1.
ly_scan:
	exx
	inc hl
	ld a,(fv_limit)
	sub (hl)
	jr nc,sn_room
	xor a
sn_room:
	ld c,a
	inc hl
	inc hl
	inc hl
	dec b
sn_loop:
	ld a,b
	or a
	jr z,sn_done
	inc hl
	ld a,(hl)
	inc hl
	add a,(hl)
	jr c,sn_break
	ld e,a
	ld a,c
	sub e
	jr c,sn_break
	ld c,a
	inc hl
	inc hl
	dec b
	jr sn_loop
sn_break:
	dec hl
	dec hl
sn_done:
	exx
	ret

; ly_scan for a line inside the window, drawing it on text row fv_rowb. The glyph codes of tokens
; one column apart are put together in FM_SCRATCH and drawn with one ui_text, which leaves one
; blank column between glyphs; a token farther from the one before it starts a run at the x after
; the last glyph plus its gap. A run is also cut after FM_RUN_MAX codes and continues one column on.
ly_draw:
	ld a,(fv_x0)
	ld l,a
	ld h,0
	ld (fv_x),hl
	ld de,FM_SCRATCH
	exx
	inc hl
	ld a,(fv_limit)
	sub (hl)
	jr nc,dw_room
	xor a
dw_room:
	ld c,a
	dec hl
	ld a,(hl)
	inc hl
	inc hl
	inc hl
	bit FM_ELLIPSIS,(hl)
	jp nz,dw_first
	inc hl
	dec b
	exx
dw_copy:
	ld hl,-FM_SCRATCH-FM_RUN_MAX
	add hl,de
	jr c,dw_cut
dw_go:
	ld l,a
	ld h,0
	add hl,hl
	ld bc,FM_GLYPHS+3
	add hl,bc
	ld c,(hl)
	inc hl
	ld b,(hl)
	ld hl,FM_GLYPHS+1
	add hl,bc
	ld c,(hl)
	inc hl
	ld b,0
	ldir
	exx
dw_next:
	ld a,b
	or a
	jr z,dw_last
	ld a,(hl)
	ex af,af'
	inc hl
	ld a,(hl)
	inc hl
	ld d,(hl)
	add a,d
	jr c,dw_wrap
	ld e,a
	ld a,c
	sub e
	jr c,dw_wrap
	ld c,a
	inc hl
	bit FM_ELLIPSIS,(hl)
	jr nz,dw_ell
	inc hl
	dec b
	ld a,d
	cp 1
	jr nz,dw_split
	ex af,af'
	exx
	jr dw_copy
dw_split:
	jr c,dw_next
	ld (fv_gap),a
	ex af,af'
	exx
	push af
	call dw_flush
	ld a,(fv_gap)
	ld l,a
	ld h,0
	add hl,de
	ld (fv_x),hl
	ld de,FM_SCRATCH
	pop af
	jr dw_copy
dw_cut:
	push af
	call dw_flush
	inc de
	ld (fv_x),de
	ld de,FM_SCRATCH
	pop af
	jr dw_go
dw_wrap:
	dec hl
	dec hl
dw_last:
	exx

; Draws the run in FM_SCRATCH, ended at DE, at fv_x. out: DE = the x after it.
dw_flush:
	xor a
	ld (de),a
	ld a,(fv_rowb)
	ld b,a
	ld hl,FM_SCRATCH
	ld de,(fv_x)
	jp ui_text

; The ellipsis of a folded node: its code joins the run as a token's codes do.
dw_first:
	inc hl
	dec b
	exx
	jr dw_put
dw_ell:
	inc hl
	dec b
	ld a,d
	cp 1
	exx
	jr z,dw_put
	ld (fv_gap),a
	call dw_flush
	ld a,(fv_gap)
	ld l,a
	ld h,0
	add hl,de
	ld (fv_x),hl
	ld de,FM_SCRATCH
dw_put:
	ld hl,-FM_SCRATCH-FM_RUN_MAX
	add hl,de
	jr nc,dw_room_ell
	call dw_flush
	inc de
	ld (fv_x),de
	ld de,FM_SCRATCH
dw_room_ell:
	ld a,UI_FONT_CH_2026
	ld (de),a
	inc de
	exx
	jp dw_next

; The line just started, number fv_line - 1: fv_show says whether it is inside the window and
; fv_rowb is its text row. out: carry when it and every later line is below the window. Keeps
; every register but AF.
fy_window:
	push hl
	push de
	push bc
	ld a,(fv_line)
	dec a
	ld e,a
	ld d,0
	ld hl,(fv_row)
	add hl,de
	ld a,(fv_hi)
	ld e,a
	push hl
	or a
	sbc hl,de
	pop hl
	jp m,fw_before_end
	xor a
	ld (fv_show),a
	scf
	jr fw_done
fw_before_end:
	ld a,(fv_lo)
	ld e,a
	push hl
	or a
	sbc hl,de
	pop hl
	ld a,0
	jp m,fw_set
	ld a,l
	ld (fv_rowb),a
	ld a,1
fw_set:
	ld (fv_show),a
	or a
fw_done:
	pop bc
	pop de
	pop hl
	ret
