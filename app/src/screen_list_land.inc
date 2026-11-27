; The theorem list's land, the land routine of kind 6 in the kind table (screens.inc): the model's
; List::land for a jump from the sections screen, running after the list's enter has derived its state
; again. Without LIBRARY the land does nothing.
;
; list_land  in: HL = the frame, DE = the assertion the title starts at. Writes 0 to byte 7 of the frame, the
;            find's length and miss flag, and touches bytes 8 to 15 not at all. Then, unless the stage has no
;            assertion or the kind shows none, puts the cursor on the first assertion at or after DE that the
;            kind shows (lf_first_ge), else on the last it shows, and the first row shown as the model's reveal
;            does from where it was: on the cursor when the cursor is above it, kept while the cursor is one of
;            the four shown after it, else four shown assertions above the cursor (lmk_below). The preview is
;            pending with LV_COUNT at LIST_WAIT when the cursor's assertion changed, and LV_PREVIEW and LV_COUNT
;            written back with the values they had when it did not. An empty stage or a kind that shows nothing
;            leaves bytes 1 to 6 of the frame and the preview alone.
;            Writes, in the frame, byte 7 always and bytes 2 to 5 when the stage has an assertion and the kind
;            shows one; LV_KIND and LV_END; LV_PREVIEW and LV_COUNT when the kind shows one; the scratch LK_FR,
;            LK_CUR, LK_WAS, LK_OLD, LK_EMPTY, LK_X, LK_N, LK_T, LK_T1 and, under Unchecked with a table, LK_AGAIN.
;            What the kind asks for writes more: the reader's LIB_STATE and FAR_VARS under Theorems and Axioms,
;            OP1 to OP5 under Unchecked with a table (the store). Destroys AF BC DE HL and IX; interrupts and
;            speed as at the call, at every exit.
; Stack, in bytes written below the stack pointer at the entry, the caller's return address counted, the
; greatest over a call with interrupts masked, at both speeds (sections_depth.rs). Stub OS, under All: 4 with
; the new cursor on or above the first row, 6 below it; under a kind 18 when the cursor lands on a later shown
; assertion, 22 when it lands on the last, 24 when the kind shows nothing under Unchecked with a table.
; Reference OS: 4 and 10 under All, 22 under a kind for a later shown assertion and for the last, 38 for the
; Unchecked case.

#ifdef LIBRARY
list_land:
	ld (LK_FR), hl
	push hl
	pop ix
	ld (LK_CUR), de
	xor a
	ld (ix + 7), a
	ld a, (ix + 1)
	call list_stage_end
	ld a, d
	or e
	ret z
	ld (LV_END), de
	ld a, (ix + 6)
	ld (LV_KIND), a
	ld l, (ix + 2)
	ld h, (ix + 3)
	ld (LK_WAS), hl
	ld a, (LV_PREVIEW)
	ld (LK_OLD), a
	ld a, (LV_COUNT)
	ld (LK_EMPTY), a
	ld hl, (LK_CUR)
	ld a, (LV_KIND)
	or a
	jr nz, ll_kind
	dec de
	push hl
	or a
	sbc hl, de
	pop hl
	jr c, ll_have
	ex de, hl
	jr ll_have
ll_kind:
	call lf_first_ge
	ld a, h
	and l
	inc a
	ret z
ll_have:
	ld (LK_X), hl
	ld ix, (LK_FR)
	ld e, (ix + 4)
	ld d, (ix + 5)
	or a
	sbc hl, de
	jr z, ll_top
	jr c, ll_above
	call lmk_below
	jr ll_settle
ll_above:
	call lmk_top_x
	jr ll_settle
ll_top:
	ld hl, (LK_X)
	ld (ix + 2), l
	ld (ix + 3), h
ll_settle:
	ld hl, (LK_X)
	ld de, (LK_WAS)
	or a
	sbc hl, de
	jr z, ll_kept
	ld a, LVP_PENDING
	ld (LV_PREVIEW), a
	ld a, LIST_WAIT
	ld (LV_COUNT), a
	ret
ll_kept:
	ld a, (LK_OLD)
	ld (LV_PREVIEW), a
	ld a, (LK_EMPTY)
	ld (LV_COUNT), a
	ret
#else
list_land:
	ret
#endif
