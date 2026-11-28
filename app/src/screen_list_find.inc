; The list's find by typed prefix (screen_list.inc has the screen's header, screen_list_key.inc the keys').
; lk_letter, lk_f2 and lk_clear answer as list_key does; list_find_again answers nothing: A is undefined at its
; return and list_kind_step loads it again. list_find looks the prefix up in the
; label order and, under the kind in force, takes the first shown assertion at or after the start whose label
; starts with it: far_lib_find_range once, then one far_lib_find_walk (All, Theorems, Axioms, and Unchecked with
; no table), or under Unchecked with a table one far_lib_find_copy and one store_least_not_one for each run of
; at most 127 entries of the range, the best so far as the limit. After a hit the first row takes at most four
; lf_next calls and, when fewer than four shown assertions follow, one lf_prev and the four of lf_back4: nine at
; most, and none of them reads anything under All.
; Statuses: a status other than 0 from far_lib_find_range, far_lib_find_walk or far_lib_find_copy is a miss; one
; from store_least_not_one shows every assertion, as lf_next does. A miss sets bit 7 of byte 7 and moves nothing.
; Writes: the frame's bytes 2 to 5 and 7 to 15, LV_PREVIEW and LV_COUNT, LV_KIND, LV_END, LK_FR, LK_CUR, LK_T,
; LK_N, LK_AGAIN (lf_next and lf_prev write it under Unchecked with a table), LF_ in LV_NAME and LV_CUT, FM_SCRATCH and the reader's and the store's state. A refused character, a
; ninth character and a letter with LV_HASO 0 write nothing. Interrupts and speed are as at the call.

#ifdef LIBRARY
; In: A = KEY_LETTER, B = the character, HL = the frame. A letter A to Z is lowercased; a to z, 0 to 9, '-',
; '_' and '.' are added to the prefix and nothing else is: a character the model refuses, a ninth
; character and every character when LV_HASO is 0 stay. Out: A = SCR_STAY, or SCR_REDRAW with the preview
; kept when the cursor's assertion is the one it was and pending with the wait when it moved.
lk_letter:
	ld a, (LV_HASO)
	or a
	jp z, lk_stay
	ld a, b
	cp 'A'
	jr c, lkl_class
	cp 'Z' + 1
	jr nc, lkl_class
	or 20h
lkl_class:
	ld c, a
	cp '-'
	jr z, lkl_take
	cp '.'
	jr z, lkl_take
	cp '_'
	jr z, lkl_take
	cp '0'
	jp c, lk_stay
	cp '9' + 1
	jr c, lkl_take
	cp 'a'
	jp c, lk_stay
	cp 'z' + 1
	jp nc, lk_stay
lkl_take:
	push hl
	pop ix
	ld a, (ix + 7)
	and LIST_FIND_LEN
	cp LIST_FIND_MAX
	jp nc, lk_stay
	ld e, a
	ld d, 0
	add hl, de
	ld de, 8
	add hl, de
	ld (hl), c
	inc (ix + 7)
	bit 7, (ix + 7)
	jr nz, lk_redraw
	ld (LK_FR), ix
	ld l, (ix + 2)
	ld h, (ix + 3)
	ld (LK_CUR), hl
	ld a, (ix + 6)
	ld (LV_KIND), a
	ld a, (ix + 1)
	call list_stage_end
	ld (LV_END), de
	ld ix, (LK_FR)
	ld a, (ix + 7)
	and LIST_FIND_LEN
	dec a
	ld hl, (LK_CUR)
	jr nz, lkl_search
	ld hl, 0
	call lf_next
lkl_search:
	call list_find
	ld ix, (LK_FR)
	ld l, (ix + 2)
	ld h, (ix + 3)
	ld de, (LK_CUR)
	or a
	sbc hl, de
	jp nz, lk_blank
lk_redraw:
	ld a, SCR_REDRAW
	ret

; In: HL = the frame. F2 with the label order section ends the find: byte 7 is cleared and the screen
; redraws, or stays when there was none.
lk_f2:
	ld a, (LV_HASO)
	or a
	jp z, lk_notice
	push hl
	pop ix
	ld a, (ix + 7)
	or a
	jp z, lk_stay
	ld (ix + 7), 0
	jr lk_redraw

; In: A = KEY_CLEAR, HL = the frame. A find ends and the screen redraws; otherwise the list pops.
lk_clear:
	push hl
	pop ix
	ld a, (ix + 7)
	or a
	jp z, lk_pop
	ld (ix + 7), 0
	jr lk_redraw

; In: LK_FR the frame, LV_KIND and LV_END set. F3 after the kind stepped: the miss bit goes, and a find
; starts again from the first assertion the new kind shows.
list_find_again:
	ld ix, (LK_FR)
	res 7, (ix + 7)
	ld a, (LV_HASO)
	or a
	ret z
	ld a, (ix + 7)
	or a
	ret z
	ld hl, 0
	call lf_next
	jp list_find

; In: LK_FR the frame, whose byte 7 has a prefix and no miss, LV_KIND and LV_END set, HL = the assertion the
; search starts at. Looks the prefix up in the label order, and on a hit moves the frame's cursor to
; the first assertion from HL on that the kind shows and whose label starts with it, and the first row
; to that assertion, or to the fifth from the last shown when fewer than four shown assertions follow
; it. A miss, or a status other than 0 from a reader call, sets the miss bit and moves nothing.
list_find:
	ld (LF_START), hl
	ld hl, (LV_END)
	ld (LF_BOUND), hl
	ld ix, (LK_FR)
	ld a, (ix + 7)
	and LIST_FIND_LEN
	ld b, a
	push ix
	pop de
	ld hl, 8
	add hl, de
	ex de, hl
	call far_lib_find_range
	or a
	jr nz, lfd_miss
	ld a, b
	or c
	jr z, lfd_miss
	ld (LF_FIRST), hl
	ld (LF_COUNT), bc
	ld b, 0FFh
	ld a, (LV_KIND)
	or a
	jr z, lfd_kind
	ld b, 1
	cp LISTK_THEOREMS
	jr z, lfd_kind
	ld b, 0
	cp LISTK_AXIOMS
	jr z, lfd_kind
	ld a, (LV_TABLE)
	or a
	jr nz, lfd_store
	ld b, 0FFh
lfd_kind:
	ld a, b
lfd_kind_a:
	ld (LF_KIND), a
lfd_walk:
	ld hl, LF_BLOCK
	call far_lib_find_walk
	or a
	jr nz, lfd_miss
lfd_answer:
	ld de, (LV_END)
	push hl
	or a
	sbc hl, de
	pop hl
	jr nz, lfd_hit
lfd_miss:
	ld ix, (LK_FR)
	set 7, (ix + 7)
	ret
lfd_store:
	ld hl, (LF_BOUND)
	ld (LF_BEST), hl
	ld hl, (LF_FIRST)
	ld (LF_PF), hl
	ld hl, (LF_COUNT)
	ld (LF_PC), hl
lfd_run:
	ld hl, (LF_PC)
	ld a, h
	or l
	jr z, lfd_done
	ld b, h
	ld c, l
	ld hl, (LF_PF)
	ld de, FM_SCRATCH
	call far_lib_find_copy
	or a
	jr nz, lfd_miss
	ld (LF_K), hl
	ld b, h
	ld c, l
	ld hl, FM_SCRATCH
	ld de, (LF_START)
	ld ix, (LF_BEST)
	call store_least_not_one
	or a
	jr nz, lfd_unmarked
	ld (LF_BEST), hl
	ld bc, (LF_K)
	ld hl, (LF_PF)
	add hl, bc
	ld (LF_PF), hl
	ld hl, (LF_PC)
	or a
	sbc hl, bc
	ld (LF_PC), hl
	jr lfd_run
lfd_done:
	ld hl, (LF_BEST)
	jr lfd_answer
lfd_unmarked:
	ld a, 0FFh
	jr lfd_kind_a

; In: HL = the assertion the cursor lands on.
lfd_hit:
	ld ix, (LK_FR)
	ld (ix + 2), l
	ld (ix + 3), h
	ld (LK_T), hl
	ld a, LIST_ROWS - 1
	ld (LK_N), a
lfd_follow:
	ld hl, (LK_T)
	inc hl
	call lf_next
	ld de, (LV_END)
	push hl
	or a
	sbc hl, de
	pop hl
	jr z, lfd_short
	ld (LK_T), hl
	ld hl, LK_N
	dec (hl)
	jr nz, lfd_follow
	ld ix, (LK_FR)
	ld e, (ix + 2)
	ld d, (ix + 3)
	jr lfd_top
lfd_short:
	ld hl, (LV_END)
	call lf_prev
	ld a, h
	and l
	inc a
	jr z, lfd_cursor
	call lf_back4
	ex de, hl
	ld ix, (LK_FR)
	ld l, (ix + 2)
	ld h, (ix + 3)
	or a
	sbc hl, de
	jr nc, lfd_top
lfd_cursor:
	ld ix, (LK_FR)
	ld e, (ix + 2)
	ld d, (ix + 3)
lfd_top:
	ld ix, (LK_FR)
	ld (ix + 4), e
	ld (ix + 5), d
	ret
#endif
