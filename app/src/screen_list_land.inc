; The theorem list's land, the land routine of kind 6 in the kind table (screens.inc): the model's
; List::land for a jump from the sections screen, running after the list's enter has derived its state
; again. Without LIBRARY the land does nothing.
;
; list_land  in: HL = the frame, DE = the assertion the title starts at. Writes 0 to byte 7 of the frame,
;            the find's length and miss flag, and touches bytes 8 to 15 not at all. Then, unless the stage
;            has no assertion or the kind shows none, puts the cursor on the first assertion at or after DE
;            that the kind shows (lf_first_ge), else on the last it shows, and the first row shown as the
;            model's reveal does from where it was: on the cursor when the cursor is above it, kept while
;            the cursor is one of the four shown after it, else four shown assertions above the cursor
;            (lmk_below; the cursor itself when the reader refuses the read of the shown assertion before the
;            cursor). The preview is pending with LV_COUNT at LIST_WAIT when the cursor's assertion changed,
;            and LV_PREVIEW and LV_COUNT are written back with the values they had when it did not. Those
;            are what list_enter has just set, pending and LIST_WAIT, on every path a user can take to the
;            land, so after every jump the preview row is blank for the wait, also when the cursor did not
;            move, where the model keeps a settled preview. An empty stage returns before writing LV_KIND
;            or LV_END, a kind that shows nothing after writing them, and both leave bytes 1 to 6 of the
;            frame, LV_PREVIEW and LV_COUNT alone.
;            Writes, in the frame, byte 7 always; bytes 2 and 3 when the cursor lands on the first row
;            and bytes 2 to 5 when it lands anywhere else, if the stage has an assertion and the kind
;            shows one; LV_KIND and LV_END unless the stage is empty; LV_PREVIEW and LV_COUNT when the kind
;            shows one; the scratch LK_FR, LK_CUR, LK_WAS, LK_OLD and LK_EMPTY (which holds LV_COUNT here,
;            not the kind step's flag its name says), LK_X when the kind shows one, LK_N, LK_T and LK_T1
;            when the walk of lmk_below runs and, under Unchecked with a table, LK_AGAIN. What the kind
;            asks for writes more: the reader's LIB_STATE and FAR_VARS under Theorems and Axioms,
;            OP1 to OP5 under Unchecked with a table (the store). Destroys AF BC DE HL and IX; interrupts
;            and speed as at the call, at every exit.
; Stack, in bytes written below the stack pointer at the entry, the caller's return address counted, the
; greatest over a call with interrupts masked, at both speeds, on each branch of the land
; (sections_land_paths.rs, sections_depth.rs). The branches are by where the cursor lands among the
; assertions the kind shows: on the first row or above it, one to five shown assertions below it (lmk_below
; asks lf_next), and six or more below it (lmk_far: call lmk_below, call lf_backn, push hl, call lf_prev,
; 8 bytes on top of the land's 2). Under All, on both OSs: 4 on the first row or above it, 6 for one to five
; below it and 10 for six or more. Under Unchecked with no table: 6, since lf_first_ge and lf_next are
; called first, and 10 for six or more. Under Theorems and Axioms, on both OSs: 18, and 22 for six or more.
; Under Unchecked with a table: 24 on the stub OS and 38 on the reference OS on every branch and when the
; kind shows nothing, and 26 and 40 for six or more below, where lf_prev asks store_prev_not_one. The
; greatest is 26 on the stub OS and 40 on the reference OS.

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
