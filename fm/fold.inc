; Folds: fm_fold and fm_folded over the node table, and the pass that applies the fold flags of
; FM_FOLDS to the token records.
;
; A fold flag per node row, in the same row as the node's FM_FIRST entry. A node is foldable when the
; node after it in its table is deeper. Of nested folded nodes only the outermost has an effect.
; The records of the selected statement hold the folds: a hidden token has width 0 and gap 0, the
; first token of a folded span is the ellipsis, marked in bit FM_ELLIPSIS of its flags byte, with
; the ellipsis width and the spacing of a variable. The pass runs when a flag changes and a
; routine needs the records, which the header's spare byte says.

; out: A = the node count of the selected statement's table, 0 and Z when nothing is selected or the
; table is not built; fv_tcnt holds the count and fv_tb the table's row in FM_FIRST.
; Destroys DE HL.
fo_table:
	ld a,(fv_count)
	or a
	ret z
	ld hl,(fv_hdr)
	ld de,3
	add hl,de
	ld a,(hl)
	or a
	ret z
	ld (fv_tcnt),a
	inc hl
	ld e,(hl)
	inc hl
	ld d,(hl)
	ld hl,FM_FIRST
	add hl,de
	ld (fv_tb),hl
	or a
	ret

; in: A = a node. out: HL = its row in the plane at offset DE from FM_FIRST. Destroys DE.
fo_row:
	ld hl,(fv_tb)
	add hl,de
	ld e,a
	ld d,0
	add hl,de
	ret

; in: A = a node below the count. out: NZ when it is foldable. Destroys AF DE HL.
fo_foldable:
	ld e,a
	inc a
	ld hl,fv_tcnt
	cp (hl)
	jr nc,ff_no
	ld a,e
	ld de,2*FM_PLANE
	call fo_row
	ld a,(hl)
	inc hl
	cp (hl)
	jr c,ff_yes
ff_no:
	xor a
	ret
ff_yes:
	or 1
	ret

; in: A = a node below the count. out: A = the node it is drawn as: itself, or the outermost folded
; node it is hidden inside. Keeps BC.
fo_effective:
	push bc
	ld c,a
	ld a,$FF
	ld (fv_m),a
	ld de,2*FM_PLANE
	xor a
	call fo_row
	push hl
	ld de,FM_PLANE
	add hl,de
	ex de,hl
	pop hl
	ld b,c
	ld c,0
ef_loop:
	ld a,(fv_m)
	inc a
	jr z,ef_free
	ld a,(fv_md)
	cp (hl)
	jr c,ef_next
	ld a,$FF
	ld (fv_m),a
ef_free:
	ld a,(de)
	or a
	jr z,ef_next
	ld a,c
	ld (fv_m),a
	ld a,(hl)
	ld (fv_md),a
ef_next:
	ld a,c
	cp b
	jr z,ef_end
	inc c
	inc hl
	inc de
	jr ef_loop
ef_end:
	ld a,(fv_m)
	inc a
	ld a,c
	jr z,ef_self
	ld a,(fv_m)
ef_self:
	pop bc
	ret

; Applies the folds to the records if the header says they are stale. Destroys all but IX, IY.
fo_fresh:
	ld a,(fv_count)
	or a
	ret z
	ld hl,(fv_hdr)
	ld de,7
	add hl,de
	ld a,(hl)
	and 1
	ret z
	ld (hl),0

; Rewrites width, gap and flags of every record of the selected statement from the symbols and the
; folds, the first token's gap against flags 0 as fm_assertion wrote it.
fo_apply:
	call fo_table
	jr nz,ap_table
	ld a,$FF
	ld (fv_fa),a
	jr ap_start
ap_table:
	xor a
	ld (fv_ni),a
	call fo_find
ap_start:
	xor a
	ld (fv_prev),a
	ld (fv_ta),a
	ld a,(fv_count)
	ld b,a
	ld hl,(fv_tok)
ap_loop:
	ld a,b
	or a
	ret z
	ld a,(fv_ta)
	ld c,a
	ld a,(fv_fa)
	cp c
	jr z,ap_ell
	jr nc,ap_plain
	ld a,(fv_fb)
	cp c
	jr nc,ap_hide
	push bc
	push hl
	call fo_find
	pop hl
	pop bc
	jr ap_loop
ap_plain:
	ld e,(hl)
	ld d,FM_SYMF>>8
	ld a,(de)
	and $0F
	ld c,a
	dec d
	ld a,(de)
	call ap_put
	jr ap_next
ap_ell:
	ld c,FM_PAD_VAR | (1<<FM_ELLIPSIS)
	ld a,(fv_ew)
	call ap_put
	jr ap_next
ap_hide:
	ld e,(hl)
	ld d,FM_SYMF>>8
	ld a,(de)
	and $0F
	ld c,a
	inc hl
	ld (hl),0
	inc hl
	ld (hl),0
	inc hl
	ld (hl),c
	inc hl
ap_next:
	ld a,(fv_ta)
	inc a
	ld (fv_ta),a
	dec b
	jr ap_loop

; in: HL = a record, A = the token's width, C = its flags. Writes the width, the gap after the previous
; shown token and the flags. out: HL = the next record. Keeps B. Destroys AF DE.
ap_put:
	inc hl
	ld (hl),a
	ld a,(fv_prev)
	ld d,a
	call fm_gap
	ld a,c
	and $0F
	ld (fv_prev),a
	inc hl
	ld (hl),e
	inc hl
	ld (hl),c
	inc hl
	ret

; Finds the next outermost folded node from node fv_ni on. out: fv_fa = its ellipsis token, fv_fb =
; the last token it hides, or fv_fa = $FF when there is none; fv_ni = the node after its subtree.
; Destroys all but IX, IY.
fo_find:
	ld a,(fv_ni)
	ld c,a
	ld de,3*FM_PLANE
	call fo_row
fi_loop:
	ld a,(fv_tcnt)
	cp c
	jr z,fi_none
	ld a,(hl)
	or a
	jr nz,fi_found
	inc hl
	inc c
	jr fi_loop
fi_none:
	ld (fv_ni),a
	ld a,$FF
	ld (fv_fa),a
	ret
fi_found:
	ld a,c
	ld de,0
	call fo_row
	ld a,(hl)
	ld b,a
	ld a,c
	or a
	jr nz,fi_first
	inc b
fi_first:
	ld a,b
	ld (fv_fa),a
	ld de,FM_PLANE
	add hl,de
	ld a,(hl)
	ld (fv_fb),a
	add hl,de
	ld d,(hl)
fi_skip:
	inc c
	ld a,(fv_tcnt)
	cp c
	jr z,fi_end
	inc hl
	ld a,(hl)
	cp d
	jr z,fi_end
	jr nc,fi_skip
fi_end:
	ld a,c
	ld (fv_ni),a
	ret

; in: E = a node, D = 0 clear, 1 set, 2 toggle
fold_body:
	call fm_need
	jp nz,fm_fail
	ld a,d
	cp 3
	ld a,FM_ERR_BAD
	jp nc,fm_fail
	ld a,e
	ld (fv_fn),a
	ld a,d
	ld (fv_fm),a
	ld hl,fold_go
	ld (fv_then),hl
	jp nodes_main

; The table is built here: A = 0, B = its node count.
fold_go:
	call fo_table
	ld a,(fv_fn)
	ld hl,fv_tcnt
	cp (hl)
	ld a,FM_ERR_INDEX
	jp nc,fm_fail
	ld a,(fv_fn)
	call fo_foldable
	ld b,0
	ld a,0
	jp z,fm_leave
	ld a,(fv_fn)
	ld de,3*FM_PLANE
	call fo_row
	ld a,(fv_fm)
	cp FM_MODE_TOGGLE
	ld c,a
	jr nz,fd_cmp
	ld a,(hl)
	xor 1
	ld c,a
fd_cmp:
	ld a,(hl)
	cp c
	ld a,0
	jp z,fm_leave
	ld (hl),c
	ld hl,(fv_hdr)
	ld de,7
	add hl,de
	ld (hl),1
	ld b,1
	xor a
	jp fm_leave

; in: E = a node. out: B = bit 0 its flag, bit 1 set when it is foldable, bit 2 set when it is hidden
; inside a folded node.
folded_body:
	call fm_need
	jp nz,fm_fail
	ld a,e
	ld (fv_fn),a
	call fo_node
	jp nz,fm_fail
	ld a,(fv_fn)
	ld de,3*FM_PLANE
	call fo_row
	ld a,(hl)
	and 1
	ld b,a
	ld a,(fv_fn)
	call fo_foldable
	jr z,fo_nofold
	set 1,b
fo_nofold:
	ld a,(fv_fn)
	call fo_effective
	ld hl,fv_fn
	cp (hl)
	jr z,fo_shown
	set 2,b
fo_shown:
	xor a
	jp fm_leave

; in: the node in fv_fn. out: Z, or NZ with A = FM_ERR_INDEX when the table is not built or has no such node.
fo_node:
	call fo_table
	ld a,FM_ERR_INDEX
	jr z,fn_bad
	ld a,(fv_fn)
	ld hl,fv_tcnt
	cp (hl)
	ld a,FM_ERR_INDEX
	jr nc,fn_bad
	xor a
	ret
fn_bad:
	or a
	ret
