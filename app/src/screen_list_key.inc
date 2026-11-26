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
;              assertion as with no table. F3 steps the kind on when LV_HASK is 1, and lands as the
;              model's F3 does: the cursor on the first shown assertion at or after the one it was on,
;              else the last, the first row shown four before it among the shown, and the preview
;              pending with the wait when the cursor's assertion changed (a list that shows nothing
;              has none), else kept. ENTER and F1 push KIND_THEOREM for the cursor's assertion
;              (theorem_push), or the notice NOTICE_LIST of the stage when the kind shows no assertion
;              (a stage with none, or a filter with no match). F2 and F4, F3 when LV_HASK is 0, and F5 when
;              LV_SECT is 1, push the notice. CLEAR pops. Every other key stays.

; In: A = the key, HL = the frame.
list_key:
	cp KEY_UP
	jr z, lk_move
	cp KEY_DOWN
	jr z, lk_move
	cp KEY_LEFT
	jr z, lk_move
	cp KEY_RIGHT
	jr z, lk_move
	cp KEY_CLEAR
	jr z, lk_pop
	cp KEY_ENTER
	jr z, lk_open
	cp KEY_F3
	jr z, lk_f3
	cp KEY_F1
	jr z, lk_open
	jr c, lk_stay
	cp KEY_F5
	jr c, lk_notice
	jr nz, lk_stay
	ld a, (LV_SECT)
	or a
	jr z, lk_stay
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
lk_pop:
	ld a, SCR_POP
	ret
lk_stay:
	ld a, SCR_STAY
	ret

lk_move:
	ld c, a
	push hl
	pop ix
	ld a, (ix + 6)
	or a
	jp nz, list_move_kind
	ld a, (ix + 1)
	call list_stage_end
	ld a, d
	or e
	jr z, lk_stay
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
	jr z, lk_stay
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
