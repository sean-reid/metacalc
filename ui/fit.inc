; Fitting text: cut or wrap a string of glyph codes to a width in pixels, as truncate, tail and
; wrap do in crates/mcui/src/canvas.rs. Every routine takes HL = the source string, DE = the
; output buffer, B = the buffer's size in bytes with the 0 that ends the result counted, and
; C = the width in pixels, 0 to 255. The source is only read and nothing is written outside
; the buffer. A result that does not fit it is refused before any byte is written. Out:
; A = 0 written, 1 refused, and the flags follow A. Each destroys AF BC DE HL.
;
; The ellipsis is UI_FONT_CH_2026 of the main font, so that table must be the one set. A glyph
; is one code, so a cut never splits one. The advance of a glyph is its width and the blank
; column after it, measured by ui_text_width, and a string fits when its advances add up to at
; most the width plus one. Each glyph is measured once, and a cut stops at the first glyph that
; does not fit. A source for ui_tail may hold some 6000 glyphs, for the others any number.
;
; ui_truncate   the string whole when it fits, else its longest prefix that fits with an
;               ellipsis after it, then the ellipsis; just the ellipsis when not even that fits
; ui_tail       the string whole when it fits, else an ellipsis then the longest suffix that fits
;               with it, down to the ellipsis alone
; ui_wrap       one line of the string cut at spaces. In: HL = where to start, the string's first
;               code for the first line. Out: HL = the end of the last word on the line, where
;               the next call starts; A = 2 when no word is left, with nothing written. HL is
;               kept when A is not 0. Runs of spaces count as one, a word is cut as ui_truncate
;               cuts it, and words join a line while it stays within the width.

TF_ELLIPSIS             .equ UI_FONT_CH_2026
TF_WORDEND              .equ SPACE_CODE+1

; tf_take   in:  A = a glyph code, not 0, DE = a budget in pixels
;           out: A = its advance; DE less that and carry clear, or DE kept and carry set when
;                the advance is more
;           destroys AF
tf_take:
	push hl
	push bc
	push de
	ld l,a
	ld h,0
	push hl
	ld hl,0
	add hl,sp
	call ui_text_width
	pop bc
	pop de
	inc l
	ld a,l
	ex de,hl
	or a
	sbc hl,de
	jr nc,fit_tk_ok
	add hl,de
fit_tk_ok:
	ex de,hl
	pop bc
	pop hl
	ret

; tf_runb   in:  HL = the end of a string, DE = budget
;           out: HL = the start of the longest tail that fits, DE = what is left
;           destroys AF
tf_runb:
	dec hl
	ld a,(hl)
	call tf_take
	jr nc,tf_runb
	inc hl
	ret

; tf_cut   in:  HL = start, DE = width + 1, A = the first code that does not end the run
;          out: HL = the end of the glyphs kept, DE = their advance with the ellipsis when one
;               follows, carry set when one follows
;          keeps BC, destroys AF
tf_cut:
	push bc
	ld c,a
	push de
	push de
	push hl
	push de
	ld de,$FFFF
	ld a,TF_ELLIPSIS
	call tf_take
	pop de
	ld b,a
fit_tc_loop:
	ld a,(hl)
	cp c
	jr c,fit_tc_whole
	call tf_take
	jr c,fit_tc_cut
	inc hl
	ld a,d
	or a
	jr nz,fit_tc_note
	ld a,e
	cp b
	jr c,fit_tc_loop
fit_tc_note:
	pop af
	pop af
	push de
	push hl
	jr fit_tc_loop
fit_tc_whole:
	pop af
	pop af
	ex (sp),hl
	or a
	sbc hl,de
	ex de,hl
	pop hl
	pop bc
	or a
	ret
fit_tc_cut:
	pop hl
	pop de
	ex (sp),hl
	or a
	sbc hl,de
	ld e,b
	ld d,0
	add hl,de
	ex de,hl
	pop hl
	pop bc
	scf
	ret

; tf_slice   in:  HL = the end of the glyphs kept, DE = their start, carry = an ellipsis too,
;                 B = the buffer size
;            out: carry set when the result and its 0 do not fit; else HL = DE's value, C = glyphs
;                 kept, B = 1 with an ellipsis, 0 without
;            destroys AF DE
tf_slice:
	ld a,l
	adc a,0
	sub e
	cp b
	ccf
	ret c
	ld b,a
	ld a,l
	sub e
	ld c,a
	ld a,b
	sub c
	ld b,a
	ex de,hl
	ret

; tf_copy   in:  HL = source, DE = destination, A = count
;           out: HL and DE after the bytes; keeps BC
;           destroys AF
tf_copy:
	or a
	ret z
	push bc
	ld c,a
	ld b,0
	ldir
	pop bc
	ret

; tf_ell   in: B = 1 for an ellipsis, DE = destination; out: DE after it; destroys AF
tf_ell:
	ld a,b
	or a
	ret z
	ld a,TF_ELLIPSIS
	ld (de),a
	inc de
	ret

tf_refuse:
	pop de
	ld a,1
	or a
	ret

ui_truncate:
	push de
	push hl
	ld e,c
	ld d,0
	inc de
	ld a,1
	call tf_cut
	pop de
fit_tt_slice:
	call tf_slice
	jr c,tf_refuse
	pop de
	ld a,c
	call tf_copy
	call tf_ell
	xor a
	ld (de),a
	ret

; tf_end   in: HL = a string; out: HL = its 0; destroys AF
tf_end:
	ld a,(hl)
	or a
	ret z
	inc hl
	jr tf_end

ui_tail:
	push de
	push hl
	push bc
	call ui_text_width
	pop bc
	pop de
	ld a,h
	or a
	jr nz,fit_tl_cut
	ld a,c
	cp l
	jr c,fit_tl_cut
	ld h,d
	ld l,e
	call tf_end
	or a
	jr fit_tt_slice
fit_tl_cut:
	ex de,hl
	call tf_end
	ld e,c
	ld d,0
	inc de
	ld a,TF_ELLIPSIS
	call tf_take
	jr c,fit_tl_none
	push hl
	call tf_runb
	ex de,hl
	pop hl
	jr fit_tl_go
fit_tl_none:
	ld d,h
	ld e,l
fit_tl_go:
	scf
	call tf_slice
	jr c,tf_refuse
	pop de
	call tf_ell
	ld a,c
	call tf_copy
	xor a
	ld (de),a
	ret

; tf_skip   in: HL = a string; out: HL = its first code that is not a space; destroys AF
tf_skip:
	ld a,(hl)
	dec a
	ret nz
	inc hl
	jr tf_skip

; tf_word   in:  HL = the start of a word, C = width
;           out: HL = the end of the word, DE = its advance as cut to the width, A = glyphs kept
;                and one for an ellipsis
;           destroys F
tf_word:
	push hl
	ld e,c
	ld d,0
	inc de
	ld a,TF_WORDEND
	call tf_cut
	ld a,l
	adc a,0
	ex (sp),hl
	sub l
	pop hl
	push af
fit_wd_skip:
	ld a,(hl)
	cp TF_WORDEND
	jr c,fit_wd_done
	inc hl
	jr fit_wd_skip
fit_wd_done:
	pop af
	ret

; tf_line   in:  HL = where the line starts, C = width
;           out: HL = the end of the last word on the line, B = the bytes the line takes without
;                its 0, 0 when no word is left; keeps C
;           destroys AF DE
tf_line:
	push hl
	ld e,c
	ld d,0
	inc de
	push de
	ld b,0
fit_tn_next:
	call tf_skip
	ld a,(hl)
	or a
	jr z,fit_tn_end
	call tf_word
	ex (sp),hl
	or a
	sbc hl,de
	jr nc,fit_tn_fit
	inc b
	dec b
	jr nz,fit_tn_end
	ld hl,0
	jr fit_tn_take
fit_tn_fit:
	ld de,3
	or a
	sbc hl,de
	jr nc,fit_tn_take
	ld hl,0
fit_tn_take:
	ld e,a
	ld a,b
	or a
	jr z,fit_tn_add
	inc a
fit_tn_add:
	add a,e
	ld b,a
	pop de
	pop af
	push de
	push hl
	ex de,hl
	jr fit_tn_next
fit_tn_end:
	pop af
	pop hl
	ret

ui_wrap:
	push de
	push hl
	ld a,b
	push af
	call tf_line
	pop af
	inc b
	dec b
	jr z,fit_wr_none
	inc b
	cp b
	jr c,fit_wr_refuse
	pop de
	ex (sp),hl
	ex de,hl
	ld b,0
	ld a,c
	sub 9
	jr c,fit_wr_loop
fit_wr_m:
	inc b
	sub 10
	jr nc,fit_wr_m
fit_wr_loop:
	call tf_skip
	push de
	push hl
	ld e,0
fit_wr_count:
	ld a,(hl)
	cp TF_WORDEND
	jr c,fit_wr_short
	inc hl
	inc e
	ld a,b
	cp e
	jr nc,fit_wr_count
	pop hl
	push hl
	ld e,c
	ld d,0
	inc de
	ld a,TF_WORDEND
	call tf_cut
	ld a,l
	pop de
	sub e
	ex de,hl
	pop de
	call tf_copy
	ld a,(hl)
	cp TF_WORDEND
	jr c,fit_wr_whole
	ld a,TF_ELLIPSIS
	ld (de),a
	inc de
	jr fit_wr_rest
fit_wr_short:
	ld a,e
	pop hl
	pop de
	call tf_copy
	jr fit_wr_whole
fit_wr_rest:
	inc hl
	ld a,(hl)
	cp TF_WORDEND
	jr nc,fit_wr_rest
fit_wr_whole:
	ex de,hl
	ex (sp),hl
	ld a,l
	cp e
	jr nz,fit_wr_more
	ld a,h
	cp d
	jr z,fit_wr_done
fit_wr_more:
	ex (sp),hl
	ex de,hl
	ld a,SPACE_CODE
	ld (de),a
	inc de
	jr fit_wr_loop
fit_wr_done:
	pop de
	xor a
	ld (de),a
	ret
fit_wr_none:
	ld a,2
	jr fit_wr_out
fit_wr_refuse:
	ld a,1
fit_wr_out:
	pop hl
	pop de
	or a
	ret
