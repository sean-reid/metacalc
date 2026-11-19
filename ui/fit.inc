; Fitting text: cut or wrap a string of glyph codes to a width in pixels, as truncate, tail and
; wrap do in crates/mcui/src/canvas.rs. Every routine takes HL = the source string, DE = the
; output buffer, B = the buffer's size in bytes with the 0 that ends the result counted, and
; C = the width in pixels, 0 to 255. The source is only read and nothing is written outside
; the buffer. A result that does not fit it is refused before any byte is written. Out:
; A = 0 written, 1 refused, and the flags follow A. Each destroys AF BC DE HL and nothing else;
; IX, IY and the shadow registers are untouched.
;
; The ellipsis is UI_FONT_CH_2026 of the main font, so that table must be the one set. A glyph
; is one code, so a cut never splits one. The advance of a glyph is its width and the blank
; column after it, from GLYPH_ADV, and a string fits when its advances add up to at most the
; width plus one. A cut reads forward until the first glyph that does not fit, then steps back
; over the few glyphs that leave no room for the ellipsis; ui_tail also measures the whole
; string once to see whether it fits; ui_wrap measures each word to place it and copies it in a
; second pass. A source for ui_tail may hold some 6000 glyphs, for the others any number.
; Only the space code, 1, separates words; the app's strings hold no other whitespace.
;
; ui_truncate   the string whole when it fits, else its longest prefix that fits with an
;               ellipsis after it, then the ellipsis; just the ellipsis when not even that fits
; ui_tail       the string whole when it fits, else an ellipsis then the longest suffix that fits
;               with it, down to the ellipsis alone
; ui_wrap       one line of the string cut at spaces, in wrap.inc. In: HL = where to start, the
;               string's first code for the first line. Out: HL = the end of the last word on
;               the line, where the next call starts; A = 2 when no word is left, with nothing
;               written. HL is kept when A is not 0. Runs of spaces count as one, a word is
;               cut as ui_truncate cuts it, and words join a line while it stays within the
;               width.
;
; The stack each needs, counting the caller's return address, is measured by the tests, which
; read these constants from the label file.
FIT_STACK_TRUNCATE      .equ 18
FIT_STACK_TAIL          .equ 12
FIT_STACK_WRAP          .equ 30

TF_ELLIPSIS             .equ UI_FONT_CH_2026
TF_WORDEND              .equ SPACE_CODE+1
TF_SPACE_ADV            .equ UI_FONT_SPACE_WIDTH+1
TF_MAX_ADV              .equ UI_FONT_MAX_WIDTH+1

; tf_take   in:  A = a glyph code, not 0, DE = a budget in pixels
;           out: A = its advance; DE less that and carry clear, or DE kept and carry set when
;                the advance is more
;           keeps HL BC, destroys F
tf_take:
	push hl
	push bc
	GLYPH_ADV
	ld c,a
	ld b,0
	ex de,hl
	or a
	sbc hl,bc
	jr nc,fit_tk_ok
	add hl,bc
fit_tk_ok:
	ex de,hl
	pop bc
	pop hl
	ret

; tf_run   in:  HL = string, DE = budget up to 256
;          out: carry set when it ran to a 0 or a space: HL there, DE what is left; carry clear
;               when a glyph did not fit: HL at it, DE what was left before it
;          destroys AF BC
tf_run:
	ld a,(hl)
	cp TF_WORDEND
	ret c
	push hl
	GLYPH_ADV
	pop hl
	ld c,a
	ld a,e
	sub c
	ld e,a
	jr nc,fit_tr_ok
	dec d
	jr nz,fit_tr_fail
fit_tr_ok:
	inc hl
	jr tf_run
fit_tr_fail:
	inc d
	ld a,e
	add a,c
	ld e,a
	or a
	ret

; tf_run_text   as tf_run, but only a 0 ends it
tf_run_text:
	ld a,(hl)
	or a
	scf
	ret z
	push hl
	GLYPH_ADV
	pop hl
	ld c,a
	ld a,e
	sub c
	ld e,a
	jr nc,fit_trt_ok
	dec d
	jr nz,fit_tr_fail
fit_trt_ok:
	inc hl
	jr tf_run_text

; tf_runb   in:  HL = the end of a string, DE = budget
;           out: HL = the start of the longest tail that fits
;           destroys AF BC DE
tf_runb:
	dec hl
	ld a,(hl)
	push hl
	GLYPH_ADV
	pop hl
	ld c,a
	ld a,e
	sub c
	ld e,a
	jr nc,tf_runb
	dec d
	jr z,tf_runb
	inc hl
	ret

; tf_cut   in:  HL = start, DE = width + 1, carry set to end at a space as well as at a 0
;          out: HL = the end of the glyphs kept, DE = their advance with the ellipsis when one
;               follows, carry set when one follows
;          keeps BC, destroys AF
tf_cut:
	push bc
	push de
	push hl
	jr c,fit_tc_word
	call tf_run_text
	jr fit_tc_ran
fit_tc_word:
	call tf_run
fit_tc_ran:
	jr nc,fit_tc_cut
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
	push hl
	ld a,TF_ELLIPSIS
	GLYPH_ADV
	ld c,a
	ld b,0
	pop hl
	ex de,hl
	or a
	sbc hl,bc
	ex de,hl
fit_tc_rw:
	bit 7,d
	jr z,fit_tc_done
	pop bc
	push bc
	ld a,l
	cp c
	jr nz,fit_tc_step
	ld a,h
	cp b
	jr z,fit_tc_done
fit_tc_step:
	dec hl
	ld a,(hl)
	push hl
	GLYPH_ADV
	pop hl
	add a,e
	ld e,a
	jr nc,fit_tc_rw
	inc d
	jr fit_tc_rw
fit_tc_done:
	pop af
	ex (sp),hl
	or a
	sbc hl,de
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
	or a
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
	push bc
	call tf_runb
	pop bc
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
