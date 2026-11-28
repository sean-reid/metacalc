; The theorem list's keys: list_key and the routines it jumps to. screen_list.inc has the screen's
; header and includes this file inside its LIBRARY branch.
;
; list_key     UP, DOWN, LEFT and RIGHT move the cursor one row or five over what the kind shows, never
;              past either end, move the first row shown as little as it must to keep the cursor on screen
;              (under a filter, four shown assertions above the cursor), answer SCR_REDRAW with
;              the preview pending and LV_COUNT at LIST_WAIT, and answer SCR_STAY when nothing moved.
;              A stay under All touches nothing. A stay under a kind leaves the frame, the preview and
;              the count as they were and writes only LV_KIND, LV_END and the scratch LK_FR, LK_CUR,
;              LK_X, LK_N and LK_KEY, and LK_AGAIN when Unchecked asks the store. Under Unchecked a
;              status other than 0 from the store, a table lost since list_enter, shows every
;              assertion as with no table. With a find typed (byte 7 of the frame not 0) an arrow clears
;              byte 7 first and then moves as above; where it moves nothing the answer is SCR_REDRAW with
;              the preview kept, since the title changes. F3 steps the kind on when LV_HASK is 1, and
;              lands as the model's F3 does: the cursor on the first shown assertion at or after the one
;              it was on, else the last, the first row shown four before it among the shown, and the
;              preview pending with the wait when the cursor's assertion changed (a list that shows
;              nothing has none), else kept. With a find typed it then clears the miss bit and searches
;              again from the first shown assertion of the new kind (screen_list_find.inc), and the
;              preview is pending when the cursor is not on the assertion it was on before the key. ENTER
;              and F1 push the notice NOTICE_LIST of the stage when it has no assertion, and under a kind
;              whenever lf_next does not answer the cursor's assertion, as with a filter with no match or
;              a refused read, and otherwise KIND_THEOREM for the cursor's assertion (theorem_push), with
;              the find left in the frame. Under a kind they write LV_KIND, LV_END and LK_FR before the
;              lf_next call. F4, F3 when LV_HASK is 0 and F2 when LV_HASO is 0 push
;              the notice. F5 when LV_SECT is 1 pushes the sections screen for the cursor's section
;              (sections_push, screen_sections.inc) and stays when it is 0. With LV_HASO 1, F2 clears byte 7 and answers SCR_REDRAW with the preview
;              kept when a find was typed, else SCR_STAY with nothing written. CLEAR with a find typed
;              clears byte 7 and answers SCR_REDRAW with the preview kept, else it pops. A letter
;              (KEY_LETTER, B the character) is the find's: see lk_letter in screen_list_find.inc. Every
;              other key stays.
;
; Stack, in bytes written below the stack pointer at the entry, the caller's return address counted, the
; greatest over a call with interrupts masked, at both speeds, taken over every cursor place of the test
; lists (list_find_exits.rs on the stub OS, list_find_depth.rs on the reference OS). F2 and CLEAR with or
; without a find, a letter after a miss and a ninth character write 4; a refused character and a letter
; with LV_HASO 0 write 2:
;                    letter (first, later, miss)   arrow or RIGHT with a find   F3 with a find
;                    stub          reference OS     stub    reference OS        stub    reference OS
;   All              20            20               6       6                   24      24
;   Theorems         20            20               22      22                  24      24
;   Axioms           20            20               22      22                  20      20
;   Unchecked, table 26            40 (miss 36)     26      40                  22      36
;   Unchecked, none  20            20               10      10                  20      20
; Over a table, F3 from Axioms into Unchecked writes 28 on the stub, which F3 from Axioms with no table does
; not.

; In: A = the key, HL = the frame.
list_key:
	or a
	jr nz, lk_dispatch
lk_stay:
	ld a, SCR_STAY
	ret
lk_dispatch:
	cp KEY_RIGHT + 1
	jp c, lk_move
	cp KEY_CLEAR
	jp z, lk_clear
	cp KEY_ENTER
	jr z, lk_open
	cp KEY_F3
	jr z, lk_f3
	cp KEY_F1
	jr z, lk_open
	jr c, lk_stay
	cp KEY_F2
	jp z, lk_f2
	cp KEY_F5
	jr c, lk_notice
	jr nz, lk_other
	ld a, (LV_SECT)
	or a
	jr z, lk_stay
	jp sections_push
lk_notice:
	inc hl
	ld c, (hl)
	ld b, NOTICE_LIST
	ld a, KIND_NOTICE
	jp screen_push_param
lk_f3:
	ld a, (LV_HASK)
	or a
	jr z, lk_notice
	jp list_kind_step
lk_open:
	push hl
	inc hl
	ld a, (hl)
	call list_stage_end
	pop hl
	ld a, d
	or e
	jr z, lk_notice
	push hl
	pop ix
	ld a, (ix + 6)
	or a
	jp z, theorem_push
	ld (LK_FR), hl
	ld (LV_KIND), a
	ld (LV_END), de
	ld l, (ix + 2)
	ld h, (ix + 3)
	call lf_next
	ld ix, (LK_FR)
	ld e, (ix + 2)
	ld d, (ix + 3)
	or a
	sbc hl, de
	ld hl, (LK_FR)
	jp z, theorem_push
	jr lk_notice
lk_other:
	cp KEY_LETTER
	jp z, lk_letter
	jp lk_stay
lk_pop:
	ld a, SCR_POP
	ret

lk_move:
	ld c, a
	push hl
	pop ix
	ld a, (ix + 7)
	or a
	jr z, lk_go
	ld (ix + 7), 0
	call lk_go
	or a
	ret nz
	jp lk_redraw
lk_go:
	ld a, (ix + 6)
	or a
	jp nz, list_move_kind
	ld a, (ix + 1)
	call list_stage_end
	ld a, d
	or e
	jp z, lk_stay
	dec de
	ld l, (ix + 2)
	ld h, (ix + 3)
	ld a, c
	cp KEY_UP
	jr z, lk_up
	cp KEY_LEFT
	jr z, lk_left
	ld bc, 1
	cp KEY_DOWN
	jr z, lk_forward
	ld c, LIST_ROWS
lk_forward:
	add hl, bc
	or a
	sbc hl, de
	jr c, lk_inside
	ex de, hl
	jr lk_moved
lk_inside:
	add hl, de
	jr lk_moved
lk_up:
	ld bc, 1
	jr lk_back
lk_left:
	ld bc, LIST_ROWS
lk_back:
	or a
	sbc hl, bc
	jr nc, lk_moved
	ld hl, 0
lk_moved:
	ld a, (ix + 2)
	cp l
	jr nz, lk_set
	ld a, (ix + 3)
	cp h
	jp z, lk_stay
lk_set:
	ld (ix + 2), l
	ld (ix + 3), h
	ld e, (ix + 4)
	ld d, (ix + 5)
	or a
	sbc hl, de
	jr c, lk_above
	ld a, h
	or a
	jr nz, lk_below
	ld a, l
	cp LIST_ROWS
	jr c, lk_blank
lk_below:
	add hl, de
	ld de, 1 - LIST_ROWS
	add hl, de
	jr lk_top
lk_above:
	add hl, de
lk_top:
	ld (ix + 4), l
	ld (ix + 5), h
lk_blank:
	ld a, LVP_PENDING
	ld (LV_PREVIEW), a
	ld a, LIST_WAIT
	ld (LV_COUNT), a
	ld a, SCR_REDRAW
	ret
