; The list's filter by kind (screen_list.inc has the kind's header). The frame's byte 6 holds the kind:
; 0 All, 1 Theorems, 2 Axioms, 3 Unchecked, in the order F3 steps through them. The cursor and the first
; row shown stay assertion numbers, and are always assertions the kind shows; a list that shows nothing
; has both at 0. Nothing here keeps a table of the matching assertions: lf_next and lf_prev ask the
; reader's bitmap of the kinds (lib_kind_next, lib_kind_prev) for Theorems and Axioms and the status
; table (store_next_not_one, store_prev_not_one) for Unchecked, and a move is a few of those calls.
; Every routine here destroys AF BC DE HL and IX. lf_next, lf_prev, lf_first_ge, lf_back4, lf_backn,
; list_kind_land and lmk_below read the stage's end from LV_END and the kind from LV_KIND, which their
; caller sets from the frame; list_kind_step, list_move_kind and list_enter_kinds set both themselves.
;
; Stack, in bytes written below the stack pointer at the entry, the caller's return address counted, the
; greatest over a call with interrupts masked (list_kind_depth.rs, list_kind_rom.rs). The arrows are the
; greatest over a whole walk. F3 is pressed under the kind named and steps to the next. Enter is list_enter
; after a notice pops with the frame as the key left it, with a cursor and a first row the kind does not
; show, and with both shown and the cursor off the screen (the fifth shown after the first row, and the
; last shown):
;   stub OS, proofs library with a table, both speeds, the whole walk
;               DOWN RIGHT UP LEFT F3 at an end enter landing below draw idle fit
;   Theorems      16   20  16  16  24    16      22     24     22    26    16
;   Axioms        16   20  16  16  28    16      22     24     22    26    16
;   Unchecked     22   24  20  20  22    22      24     28     26    26    22
;   reference OS, prop with a table, 6 MHz, at most 40 keys in each direction
;   Theorems      16   20  16  16  24     -      36     36     36    34    16
;   Axioms        16   20  16  16  42    16      36     36     36    34    16
;   Unchecked     36   38  34  34  36     -      38     42     40    36    36
; A dash is a place the 40 keys do not reach. ENTER and F1 write what DOWN does, the greatest of the walk,
; since they ask lf_next whether the kind shows the cursor before they open its theorem. Under each kind
; the idle call settled takes 2, CLEAR and F2 4, a letter 20 (Unchecked with a table: 26 on the stub, 40 on the
; reference OS), F4 6, F5 2 where no title starts in the stage (with titles 18 under Theorems and Axioms on both OSs and, under Unchecked, 22 on the stub and 36 on the reference OS, sections_push's), a tick 8 on the stub and 14 on the reference OS. An interrupt that lands in a call can only add to these: the figures with the app's
; interrupts on are never below the masked ones.

LISTK_ALL	.equ 0
LISTK_THEOREMS	.equ 1
LISTK_AXIOMS	.equ 2
LISTK_UNCHECKED	.equ 3

#ifdef LIBRARY
; In: HL = a number at most LV_END. Out: HL = the first assertion from there on that the kind shows, below
; LV_END, or LV_END when none is. With no status table every assertion is unchecked, and the store is not
; asked; when the store answers a status other than 0 (a table lost since the list was entered) every
; assertion is unchecked too. A refused reader read answers LV_END.
lf_next:
	ld a, (LV_KIND)
	or a
	ret z
	cp LISTK_UNCHECKED
	jr nz, lfn_reader
	ld a, (LV_TABLE)
	or a
	ret z
	ld de, (LV_END)
	ld (LK_AGAIN), hl
	call store_next_not_one
	or a
	ret z
	ld hl, (LK_AGAIN)
	ret
lfn_reader:
	ld b, 1
	cp LISTK_THEOREMS
	jr z, lfn_ask
	ld b, 0
lfn_ask:
	ld de, (LV_END)
	call far_lib_kind_next
	or a
	ret z
lfn_end:
	ld hl, (LV_END)
	ret

; In: HL = a number at most LV_END. Out: HL = the last assertion below it that the kind shows, or FFFFh.
; Unchecked treats every assertion as shown with no table and when the store answers a status other than 0.
lf_prev:
	ld a, (LV_KIND)
	or a
	jr z, lfp_all
	cp LISTK_UNCHECKED
	jr nz, lfp_reader
	ld a, (LV_TABLE)
	or a
	jr z, lfp_all
	ld (LK_AGAIN), hl
	call store_prev_not_one
	or a
	ret z
	ld hl, (LK_AGAIN)
	jr lfp_all
lfp_reader:
	ld b, 1
	cp LISTK_THEOREMS
	jr z, lfp_ask
	ld b, 0
lfp_ask:
	call far_lib_kind_prev
lfp_status:
	or a
	ret z
lfp_none:
	ld hl, 0FFFFh
	ret
lfp_all:
	ld a, h
	or l
	jr z, lfp_none
	dec hl
	ret

; In: HL = a number at most LV_END. Out: HL = the first assertion at or after it that the kind shows,
; else the last one it shows, or FFFFh when it shows none.
lf_first_ge:
	call lf_next
	ld de, (LV_END)
	push hl
	or a
	sbc hl, de
	pop hl
	ret nz
	ex de, hl
	jr lf_prev

; In: HL = an assertion the kind shows. Out: HL = the one four before it among those, or the first one
; when fewer come before. lf_backn does A steps back instead of four.
lf_back4:
	ld a, 4
lf_backn:
	ld (LK_N), a
lb4_step:
	push hl
	call lf_prev
	ld a, h
	and l
	inc a
	jr z, lb4_first
	pop af
	ld a, (LK_N)
	dec a
	ld (LK_N), a
	jr nz, lb4_step
	ret
lb4_first:
	pop hl
	ret

; In: LV_KIND and LV_END set, LK_FR the frame. Moves the cursor to the first assertion at or after it that
; the kind shows, else to the last, and the first row shown to the one four before the cursor among the
; shown assertions, or the first. Out: A = 1 when the kind shows nothing, with both at 0, else 0.
list_kind_land:
	ld ix, (LK_FR)
	ld l, (ix + 2)
	ld h, (ix + 3)
	call lf_first_ge
	ld a, h
	and l
	inc a
	jr z, lkl_empty
	push hl
	call lf_back4
	ex de, hl
	pop hl
	ld c, 0
	jr lkl_set
lkl_empty:
	ld hl, 0
	ld d, h
	ld e, l
	ld c, 1
lkl_set:
	ld ix, (LK_FR)
	ld (ix + 2), l
	ld (ix + 3), h
	ld (ix + 4), e
	ld (ix + 5), d
	ld a, c
	ret

; In: HL = the frame. F3: the kind steps on and the frame lands as list_kind_land says. The preview stays
; when the cursor's assertion is the one it was, which for a list that shows nothing is none: it is pending
; again, with the wait, when the assertion changed or the list gained or lost its rows.
list_kind_step:
	ld (LK_FR), hl
	push hl
	pop ix
	ld a, (ix + 1)
	call list_stage_end
	ld (LV_END), de
	ld a, (ix + 6)
	ld (LK_OLD), a
	inc a
	and 3
	ld (ix + 6), a
	ld (LV_KIND), a
	ld l, (ix + 2)
	ld h, (ix + 3)
	ld (LK_WAS), hl
	call list_kind_land
	ld (LK_EMPTY), a
	call list_find_again
	ld a, (LK_EMPTY)
	or a
	jr nz, lks_same
	ld ix, (LK_FR)
	ld l, (ix + 2)
	ld h, (ix + 3)
	ld de, (LK_WAS)
	or a
	sbc hl, de
	jr nz, lks_changed
lks_same:
	ld a, (LK_OLD)
	ld (LV_KIND), a
	ld hl, 0
	call lf_next
	ld de, (LV_END)
	or a
	sbc hl, de
	ld a, 0
	jr nz, lks_old
	inc a
lks_old:
	ld hl, LK_EMPTY
	xor (hl)
	jr z, lks_done
lks_changed:
	ld a, LVP_PENDING
	ld (LV_PREVIEW), a
	ld a, LIST_WAIT
	ld (LV_COUNT), a
lks_done:
	ld a, SCR_REDRAW
	ret

; In: C = KEY_UP, KEY_DOWN, KEY_LEFT or KEY_RIGHT, IX = the frame, whose kind is not All. Moves the cursor
; over the assertions the kind shows, one or LIST_ROWS, never past the first or the last, and answers as
; list_key does. The first row shown moves as little as it must: to the cursor when the cursor went above it,
; else, when the cursor is more than four shown assertions below it, to the one four before the cursor.
list_move_kind:
	ld (LK_FR), ix
	ld a, c
	ld (LK_KEY), a
	ld a, (ix + 1)
	call list_stage_end
	ld (LV_END), de
	ld a, (ix + 6)
	ld (LV_KIND), a
	ld l, (ix + 2)
	ld h, (ix + 3)
	ld (LK_CUR), hl
	ld (LK_X), hl
	ld a, (LK_KEY)
	ld b, 1
	cp KEY_DOWN
	jr z, lmk_forward
	cp KEY_UP
	jr z, lmk_backward
	ld b, LIST_ROWS
	cp KEY_RIGHT
	jr nz, lmk_backward
lmk_forward:
	ld a, b
	ld (LK_N), a
lmk_fstep:
	ld hl, (LK_X)
	inc hl
	call lf_next
	ex de, hl
	ld hl, (LV_END)
	or a
	sbc hl, de
	jr z, lmk_moved
	ld (LK_X), de
	ld hl, LK_N
	dec (hl)
	jr nz, lmk_fstep
	jr lmk_moved
lmk_backward:
	ld a, b
	ld (LK_N), a
lmk_bstep:
	ld hl, (LK_X)
	call lf_prev
	ld a, h
	and l
	inc a
	jr z, lmk_moved
	ld (LK_X), hl
	ld hl, LK_N
	dec (hl)
	jr nz, lmk_bstep
lmk_moved:
	ld hl, (LK_X)
	ld de, (LK_CUR)
	or a
	sbc hl, de
	jp z, lk_stay
	ld ix, (LK_FR)
	ld e, (ix + 4)
	ld d, (ix + 5)
	jr nc, lmk_below
	ld hl, (LK_X)
	or a
	sbc hl, de
	jr c, lmk_top_x
	jr lmk_commit
lmk_top_x:
	ld de, (LK_X)
	jr lmk_commit
; In: DE = the first row shown, LK_X = a shown assertion below it, LK_FR, LV_KIND and LV_END set. The first
; row stays when LK_X is one of the four shown after it. When LK_X is the fifth, the first row becomes the
; first of those four, which the walk kept in LK_T1, since the shown assertion before LK_X is the fourth.
; Otherwise it is the one four before LK_X among the shown, found from that one, or LK_X itself when the
; reader refuses the read of the shown assertion before it. Commits the frame.
lmk_below:
	ld (LK_T), de
	ld a, LIST_ROWS - 1
	ld (LK_N), a
lmk_walk:
	ld hl, (LK_T)
	inc hl
	call lf_next
	ld a, (LK_N)
	cp LIST_ROWS - 1
	jr nz, lmk_not_first
	ld (LK_T1), hl
lmk_not_first:
	ld de, (LK_X)
	push hl
	or a
	sbc hl, de
	pop hl
	jr z, lmk_inside
	ld (LK_T), hl
	ld hl, LK_N
	dec (hl)
	jr nz, lmk_walk
	ld hl, (LK_X)
	call lf_prev
	ld de, (LK_T)
	push hl
	or a
	sbc hl, de
	pop hl
	jr nz, lmk_far
	ld de, (LK_T1)
	jr lmk_commit
lmk_far:
	ld a, h
	and l
	inc a
	jr z, lmk_top_x
	ld a, LIST_ROWS - 2
	call lf_backn
	ex de, hl
	jr lmk_commit
lmk_inside:
	ld ix, (LK_FR)
	ld e, (ix + 4)
	ld d, (ix + 5)
lmk_commit:
	ld ix, (LK_FR)
	ld hl, (LK_X)
	ld (ix + 2), l
	ld (ix + 3), h
	ld (ix + 4), e
	ld (ix + 5), d
	ld a, LVP_PENDING
	ld (LV_PREVIEW), a
	ld a, LIST_WAIT
	ld (LV_COUNT), a
	ld a, SCR_REDRAW
	ret

; In: LK_FR the frame. Finds out whether the library has the kinds section, which F3 needs, and for a frame
; whose kind is not All puts the cursor and the first row shown back on assertions the kind shows, as F3
; would: the marks may have changed since the frame was made. A cursor that is not shown lands as F3 lands it,
; and so does a first row that is not shown. When both are shown, a cursor above the first row gets the first
; row on the cursor; otherwise the walk of list_move_kind goes from the first row: a cursor that is one of the
; five shown from it stays, any other gets the first row four shown assertions above it.
list_enter_kinds:
	call far_lib_sections
	or a
	ret nz
	ld a, b
	and LIST_LO_SECTION
	jr z, lek_kinds
	ld a, 1
	ld (LV_HASO), a
lek_kinds:
	ld a, b
	and LIST_KS_SECTION
	ret z
	ld a, 1
	ld (LV_HASK), a
	ld ix, (LK_FR)
	ld a, (ix + 6)
	or a
	ret z
	ld (LV_KIND), a
	ld a, (ix + 1)
	call list_stage_end
	ld (LV_END), de
	ld l, (ix + 2)
	ld h, (ix + 3)
	call lf_next
	ld ix, (LK_FR)
	ld e, (ix + 2)
	ld d, (ix + 3)
	or a
	sbc hl, de
	jr nz, lek_land
	ld l, (ix + 4)
	ld h, (ix + 5)
	call lf_next
	ld ix, (LK_FR)
	ld e, (ix + 4)
	ld d, (ix + 5)
	or a
	sbc hl, de
	jr nz, lek_land
	ld l, (ix + 2)
	ld h, (ix + 3)
	ld (LK_X), hl
	or a
	sbc hl, de
	ret z
	jp c, lmk_top_x
	jp lmk_below
lek_land:
	jp list_kind_land
#endif
