; ui_wrap: one line of a string of glyph codes cut at spaces, built on tf_cut; see fit.inc.

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
	scf
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
	ld de,TF_SPACE_ADV
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
	sub TF_MAX_ADV-1
	jr c,fit_wr_loop
fit_wr_m:
	inc b
	sub TF_MAX_ADV
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
	scf
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
