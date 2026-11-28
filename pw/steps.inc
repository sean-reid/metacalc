; The steps of the proof, one unit each, inside the session: port 7 shows the theorem's page, HL
; walks the proof, and the stack of entries, the saved entries and the goal table are on the spare
; page. A stack entry is a goal's number or PW_SYNTAX, and the address of the first step of what
; made it.

; in: A = an entry, DE = the address of its first step. Pushes it, or leaves with PW_ERR_STACK when
; 256 entries are there already.
pw_push:
	ld c,a
	ld hl,(pv_nstack)
	ld a,h
	or a
	jr nz,pwn_stack_full
	push de
	ld de,PW_ENTRIES
	add hl,de
	ld (hl),c
	ld hl,(pv_nstack)
	add hl,hl
	ld de,PW_STARTS
	add hl,de
	pop de
	ld (hl),e
	inc hl
	ld (hl),d
	ld hl,(pv_nstack)
	inc hl
	ld (pv_nstack),hl
	ret
pwn_stack_full:
	ld a,PW_ERR_STACK
	jp pw_leave

; Stores the two bytes at `src` in the row of `col` that pv_t2 (twice the goal's number) names.
#macro PW_STORE2(col, src)
	ld hl,(pv_t2)
	ld de,col
	add hl,de
	ld de,(src)
	ld (hl),e
	inc hl
	ld (hl),d
#endmacro

; in: pv_acc = the sizes of the new goal's subgoals added up, pv_step, pv_begin, pv_by and pv_ls.
; out: A = the new goal's number, its size, one more than pv_acc, and its row of the table stored.
; Leaves with PW_ERR_GOALS when 255 goals exist and with PW_ERR_TOTAL when the size passes 24 bits.
pw_mkgoal:
	ld a,(pv_goals)
	cp PW_MAX_GOALS
	jr nc,pwn_goals
	ld hl,(pv_acc)
	ld de,1
	add hl,de
	ld a,(pv_acc+2)
	adc a,0
	jr c,pwn_total
	ld (pv_acc),hl
	ld (pv_acc+2),a
	ld a,(pv_goals)
	call pw_sizeptr
	ex de,hl
	ld hl,pv_acc
	ld bc,3
	ldir
	ld a,(pv_goals)
	ld l,a
	ld h,0
	add hl,hl
	ld (pv_t2),hl
	PW_STORE2(PW_STEP, pv_step)
	PW_STORE2(PW_BEGIN, pv_begin)
	PW_STORE2(PW_BY, pv_by)
	PW_STORE2(PW_LSTART, pv_ls)
	ld a,(pv_goals)
	ld hl,pv_goals
	inc (hl)
	ret
pwn_goals:
	ld a,PW_ERR_GOALS
	jp pw_leave
pwn_total:
	ld a,PW_ERR_TOTAL
	jp pw_leave

; in: A = a goal's number. out: HL = the address of its size.
pw_sizeptr:
	ld l,a
	ld h,0
	ld e,l
	ld d,h
	add hl,hl
	add hl,de
	ld de,PW_SIZES
	add hl,de
	ret

; in: A = a subgoal's number. Adds it to the links of the goal being made and its size to pv_acc.
; Leaves with PW_ERR_SUBGOALS for a 32nd subgoal, PW_ERR_LINKS when the links table is full and
; PW_ERR_TOTAL when the size passes 24 bits.
pw_link:
	ld c,a
	ld a,(pv_nc)
	cp PW_MAX_SUBGOALS
	jr c,pwn_room
	ld a,PW_ERR_SUBGOALS
	jp pw_leave
pwn_room:
	ld hl,(pv_links)
	ld de,PW_MAX_LINKS
	or a
	sbc hl,de
	jr c,pwn_fits
	ld a,PW_ERR_LINKS
	jp pw_leave
pwn_fits:
	ld hl,(pv_links)
	ld de,PW_LINKS
	add hl,de
	ld (hl),c
	ld hl,(pv_links)
	inc hl
	ld (pv_links),hl
	ld hl,pv_nc
	inc (hl)
	ld a,c
	call pw_sizeptr
	ld a,(pv_acc)
	add a,(hl)
	ld (pv_acc),a
	inc hl
	ld a,(pv_acc+1)
	adc a,(hl)
	ld (pv_acc+1),a
	inc hl
	ld a,(pv_acc+2)
	adc a,(hl)
	ld (pv_acc+2),a
	ret nc
	ld a,PW_ERR_TOTAL
	jp pw_leave

; Takes the steps from pv_pp on, or leaves with the status that ends the walk.
pw_proof:
	ld a,(pv_rpage)
	out (7),a
pwp_loop:
	ld hl,(pv_pp)
	ld de,(pv_pend)
	or a
	sbc hl,de
	jp nc,pwp_end
	call pw_unit
	ld hl,(pv_pp)
	ld (pv_step),hl
	ld a,(hl)
	inc hl
	cp $FF
	jp z,pwp_save
	cp $F0
	jr c,pwp_small
	sub $F0
	ld b,a
	ld de,(pv_pend)
	push hl
	or a
	sbc hl,de
	pop hl
	jr c,pwp_wide
	ld a,PW_ERR_CUT
	jp pw_leave
pwp_wide:
	ld c,(hl)
	inc hl
	ld (pv_pp),hl
	ld hl,$F0
	add hl,bc
	ex de,hl
	jr pwp_number
pwp_small:
	ld (pv_pp),hl
	ld e,a
	ld d,0
pwp_number:
	ld hl,(pv_steps)
	inc hl
	ld (pv_steps),hl
	ld a,(pv_nhyps)
	ld c,a
	ld b,0
	ld h,d
	ld l,e
	or a
	sbc hl,bc
	jr c,pwp_hyp
	ld bc,(pv_nrefs)
	ld d,h
	ld e,l
	or a
	sbc hl,bc
	jr c,pwp_ref
	ld de,(pv_nsaved)
	push hl
	or a
	sbc hl,de
	pop hl
	jr c,pwp_reuse
	ld a,PW_ERR_BAD_REF
	jp pw_leave
pwp_reuse:
	ld de,PW_SAVED
	add hl,de
	ld a,(hl)
	ld de,(pv_step)
	call pw_push
	jp pwp_loop

pwp_hyp:
	ld hl,PW_HYPK
	add hl,de
	ld a,(hl)
	or a
	jr z,pwp_syntax
	ld c,a
	ld a,e
	ld hl,pv_nfl
	cp (hl)
	jr nc,pwp_essential
	ld c,0
pwp_essential:
	ld b,$80
	ld (pv_by),bc
	ld hl,(pv_step)
	ld (pv_begin),hl
	ld hl,(pv_links)
	ld (pv_ls),hl
	xor a
	ld (pv_acc),a
	ld (pv_acc+1),a
	ld (pv_acc+2),a
	call pw_mkgoal
	ld de,(pv_step)
	call pw_push
	jp pwp_loop
pwp_syntax:
	ld a,PW_SYNTAX
	ld de,(pv_step)
	call pw_push
	jp pwp_loop

pwp_ref:
	ld hl,(pv_refs)
	add hl,de
	add hl,de
	ld c,(hl)
	inc hl
	ld b,(hl)
	bit 7,b
	jr z,pwp_by
	ld bc,$8000
pwp_by:
	ld (pv_by),bc
	ld hl,PW_MEMO
	add hl,de
	add hl,de
	ld b,(hl)
	inc hl
	ld c,(hl)
	ld hl,(pv_nstack)
	ld e,b
	ld d,0
	or a
	sbc hl,de
	jr nc,pwp_pop
	ld a,PW_ERR_UNDERFLOW
	jp pw_leave
pwp_pop:
	ld (pv_t1),hl
	ld a,b
	or a
	jr z,pwp_own
	add hl,hl
	ld de,PW_STARTS
	add hl,de
	ld e,(hl)
	inc hl
	ld d,(hl)
	ex de,hl
	jr pwp_begun
pwp_own:
	ld hl,(pv_step)
pwp_begun:
	ld (pv_begin),hl
	ld a,c
	or a
	jr z,pwp_ref_syntax
	xor a
	ld (pv_acc),a
	ld (pv_acc+1),a
	ld (pv_acc+2),a
	ld (pv_nc),a
	ld hl,(pv_links)
	ld (pv_ls),hl
	ld hl,(pv_t1)
	ld (pv_t2),hl
pwp_child:
	ld hl,(pv_t2)
	ld de,(pv_nstack)
	or a
	sbc hl,de
	jr nc,pwp_made
	ld hl,(pv_t2)
	ld de,PW_ENTRIES
	add hl,de
	ld a,(hl)
	ld hl,(pv_t2)
	inc hl
	ld (pv_t2),hl
	cp PW_SYNTAX
	jr z,pwp_child
	call pw_link
	jr pwp_child
pwp_made:
	call pw_mkgoal
	ld hl,(pv_t1)
	ld (pv_nstack),hl
	ld de,(pv_begin)
	call pw_push
	jp pwp_loop
pwp_ref_syntax:
	ld hl,(pv_t1)
	ld (pv_nstack),hl
	ld a,PW_SYNTAX
	ld de,(pv_begin)
	call pw_push
	jp pwp_loop

pwp_save:
	ld (pv_pp),hl
	ld hl,(pv_nstack)
	ld a,h
	or l
	jr nz,pwp_saving
	ld a,PW_ERR_SAVE_EMPTY
	jp pw_leave
pwp_saving:
	ld de,(pv_nsaved)
	ld a,d
	cp 2
	jr c,pwp_room
	ld a,PW_ERR_SAVED
	jp pw_leave
pwp_room:
	dec hl
	ld bc,PW_ENTRIES
	add hl,bc
	ld a,(hl)
	ld hl,PW_SAVED
	add hl,de
	ld (hl),a
	inc de
	ld (pv_nsaved),de
	jp pwp_loop

pwp_end:
	ld hl,(pv_nstack)
	ld a,h
	or l
	jr nz,pwe_some
	ld a,PW_ERR_NO_ENTRY
	jp pw_leave
pwe_some:
	dec hl
	ld a,h
	or l
	jr z,pwe_one
	ld a,PW_ERR_MANY_ENTRIES
	jp pw_leave
pwe_one:
	ld a,(PW_ENTRIES)
	cp PW_SYNTAX
	jr nz,pwe_goal
	ld a,PW_ERR_SYNTAX
	jp pw_leave
pwe_goal:
	call pw_sizeptr
	ld de,pv_total
	ld bc,3
	ldir
	xor a
	jp pw_leave
