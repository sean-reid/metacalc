; The theorem screen's key (screen_theorem.inc has the kind's header): the model's press over fm_step, fm_folded,
; fm_fold, fm_nodes and fm_span.

#ifdef LIBRARY
; In: A = the key, HL = the frame.
theorem_key:
	ld c, a
	ld a, (TV_LARGE)
	or a
	jr z, tk_main
	ld a, c
	cp KEY_F2
	jr z, tk_notice
	cp KEY_F4
	jr z, tk_notice
	cp KEY_CLEAR
	jr z, tk_pop
	jr tk_stay
tk_main:
	ld a, c
	cp KEY_CLEAR
	jr z, tk_pop
	cp KEY_F2
	jr c, tk_work
	cp KEY_F5
	jr c, tk_notice
	push hl
	ld de, 8
	add hl, de
	ld a, (hl)
	pop hl
	or a
	jr nz, tk_stay
	ld c, 0
	jr tk_work
tk_stay:
	ld a, SCR_STAY
	ret
tk_pop:
	ld a, SCR_POP
	ret
tk_notice:
	inc hl
	ld c, (hl)
	ld b, NOTICE_LIST
	ld a, KIND_NOTICE
	jp screen_push_param
tk_work:
	ld a, c
	ld (TV_KEY), a
	ld (TV_FRAME), hl
	ld de, 4
	add hl, de
	ld de, TV_S
	ld bc, 5
	ldir
	ld hl, TV_S
	ld de, TV_OLD
	ld bc, 4
	ldir
	ld a, 1
	ld (TV_DOFOLLOW), a
	xor a
	ld (TV_CHG), a
	ld a, (TV_S)
	call theorem_select
	jp nz, tk_fail
	ld a, (TV_KEY)
	or a
	jp z, tk_done
	cp KEY_ENTER
	jp z, tk_fold
	cp KEY_F1
	jp z, tk_fold
	cp KEY_UP
	jp z, tk_up
	cp KEY_DOWN
	jp z, tk_down
	cp KEY_LEFT
	ld a, 3
	jr z, tk_sibling
	ld a, 2
tk_sibling:
	ld (TV_ROW), a
	ld a, (TV_N)
	ld e, a
	ld a, (TV_ROW)
	ld d, a
	call far_fm_step
	or a
	jp nz, tk_fail
	ld a, b
	inc a
	jr z, tk_none
	dec a
	ld (TV_N), a
	jp tk_done
tk_none:
	ld a, (TV_N)
	or a
	jp nz, tk_done
	ld a, (TV_ROW)
	cp 3
	ld a, (TV_S)
	jr z, tk_before
	ld hl, TV_NHYPS
	cp (hl)
	jp nc, tk_done
	inc a
	jr tk_to
tk_before:
	or a
	jp z, tk_done
	dec a
tk_to:
	ld (TV_S), a
	jp tk_done
tk_fail:
	ld c, a
	ld b, NOTICE_ERROR
	ld a, KIND_NOTICE
	jp screen_push_param
tk_fold:
	ld a, (TV_N)
	ld e, a
	ld d, 2
	call far_fm_fold
	or a
	jp nz, tk_fail
	ld a, b
	or a
	jp z, tk_done
	ld a, (TV_N)
	ld e, a
	call far_fm_folded
	or a
	jp nz, tk_fail
	bit 2, b
	jp nz, tk_done
	ld a, 1
	ld (TV_CHG), a
	call theorem_recount
	jp nz, tk_fail
	jp tk_done
tk_down:
	ld a, (TV_N)
	ld e, a
	call far_fm_folded
	or a
	jp nz, tk_fail
	bit 0, b
	jr nz, tk_one
	ld a, (TV_N)
	ld e, a
	ld d, 1
	call far_fm_step
	or a
	jp nz, tk_fail
	ld a, b
	inc a
	jr z, tk_one
	dec a
	ld (TV_N), a
	jp tk_done
tk_one:
	call far_fm_nodes
	or a
	jp nz, tk_fail
	ld a, b
	dec a
	jp nz, tk_done
	ld (TV_DOFOLLOW), a
	ld hl, (TV_SCROLL)
	ld de, THM_ROWS
	add hl, de
	ld de, (TV_TOTAL)
	or a
	sbc hl, de
	jp nc, tk_done
	ld hl, (TV_SCROLL)
	inc hl
	ld (TV_SCROLL), hl
	jp tk_done
tk_up:
	ld a, (TV_N)
	ld e, a
	ld d, 0
	call far_fm_step
	or a
	jp nz, tk_fail
	ld a, b
	inc a
	jr z, tk_back
	dec a
	ld (TV_N), a
	jp tk_done
tk_back:
	call far_fm_nodes
	or a
	jp nz, tk_fail
	ld a, b
	dec a
	jp nz, tk_done
	ld (TV_DOFOLLOW), a
	ld hl, (TV_SCROLL)
	ld a, h
	or l
	jp z, tk_done
	dec hl
	ld (TV_SCROLL), hl
tk_done:
	ld a, (TV_CHG)
	or a
	jr nz, tk_finish
	ld a, (TV_FOLLOWING)
	or a
	jr z, tk_finish
	call tk_moved
	jp z, tk_stay
tk_finish:
	call theorem_finish
	jp nz, tk_fail
	ld a, (TV_CHG)
	or a
	jr nz, tk_changed
	ld a, (TV_S)
	ld hl, TV_OLD
	cp (hl)
	jr nz, tk_changed
	ld hl, (TV_SCROLL)
	ld de, (TV_OLD + 2)
	or a
	sbc hl, de
	jr nz, tk_changed
	ld a, (TV_N)
	ld hl, TV_OLD + 1
	cp (hl)
	jr z, tk_stayed
	call theorem_folded_bits
	jp nz, tk_fail
	bit 2, b
	jr nz, tk_hidden
	ld a, (TV_KEY)
	cp KEY_UP
	jr nz, tk_changed
	bit 0, b
	jr nz, tk_stayed
tk_changed:
	call theorem_store
	ld a, SCR_REDRAW
	ret
tk_hidden:
	ld a, (TV_N)
	call theorem_drawn
	jp nz, tk_fail
	push bc
	ld a, (TV_OLD + 1)
	call theorem_drawn
	pop de
	jp nz, tk_fail
	ld a, b
	cp d
	jr nz, tk_changed
tk_stayed:
	call theorem_store
	ld a, SCR_STAY
	ret

; Out: Z when TV_S, TV_N and TV_SCROLL are as TV_OLD holds them. Destroys AF BC DE HL.
tk_moved:
	ld hl, TV_S
	ld de, TV_OLD
	ld b, 4
tk_cmp:
	ld a, (de)
	cp (hl)
	ret nz
	inc hl
	inc de
	djnz tk_cmp
	ret

; Out: A = a status, NZ when not 0; B = fm_folded's bits for the cursor's node. Destroys all.
theorem_folded_bits:
	ld a, (TV_N)
	ld e, a
	call far_fm_folded
	or a
	ret

; In: A = a node of the selected statement. Out: A = a status, NZ when not 0; B = the node the cursor
; is drawn as, the node itself or, for a node hidden inside a fold, its nearest ancestor that is not
; hidden. Destroys all.
theorem_drawn:
	ld (TV_I), a
	ld e, a
	call far_fm_folded
	or a
	ret nz
	bit 2, b
	jr nz, tw_up
	ld a, (TV_I)
	ld b, a
	xor a
	ret
tw_up:
	ld a, (TV_I)
	ld e, a
	ld d, 0
	call far_fm_step
	or a
	ret nz
	ld a, b
	jr theorem_drawn
#else
theorem_key:
	xor a
	ret
#endif
