; The two passes before the first step, each a unit of work for every entry it reads: the hypotheses
; of the theorem and the references it lists. They run inside the session, on the private stack,
; with a library page in bank B and the tables on the spare page.

; The hypotheses: for each, in the table of ranks, 0 for a statement that is not a goal (a float
; hypothesis, or an essential one that starts with another typecode), else the rank of an essential
; one among the essential ones, from 1.
pw_hyps:
	ld a,(pv_rpage)
	out (7),a
pwg_loop:
	ld a,(pv_hrem)
	or a
	jr z,pwg_done
	call pw_unit
	ld hl,(pv_hyps)
	ld a,(hl)
	inc hl
	or a
	jr nz,pwg_ess
	inc hl
	ld (pv_hyps),hl
	xor a
	jr pwg_store
pwg_ess:
	ld c,(hl)
	inc hl
	ld b,0
	ld a,(pv_rank)
	inc a
	ld (pv_rank),a
	ld e,a
	ld a,c
	or a
	jr z,pwg_not
	ld a,(pv_tc)
	cp (hl)
	jr z,pwg_have
pwg_not:
	ld e,0
pwg_have:
	add hl,bc
	ld (pv_hyps),hl
	ld a,e
pwg_store:
	ld hl,(pv_hk)
	ld (hl),a
	inc hl
	ld (pv_hk),hl
	ld hl,pv_hrem
	dec (hl)
	jr pwg_loop
pwg_done:
	ld a,1
	ld (pv_stage),a
	jp pw_run

; The references: for each, in the memo, the hypothesis count of the assertion it names and 1 when
; its conclusion starts with the provable typecode; a dummy variable gives 0 and 0.
pw_refs:
	ld a,(pv_rpage)
	out (7),a
pwr_loop:
	ld hl,(pv_ri)
	ld a,h
	or l
	jr z,pwr_done
	call pw_unit
	ld hl,(pv_ri)
	dec hl
	ld (pv_ri),hl
	ld hl,(pv_rp)
	ld e,(hl)
	inc hl
	ld d,(hl)
	inc hl
	ld (pv_rp),hl
	bit 7,d
	jr nz,pwr_dummy
	ld hl,(pv_id)
	ex de,hl
	or a
	sbc hl,de
	jr c,pwr_earlier
	ld a,PW_ERR_FORWARD
	jp pw_leave
pwr_earlier:
	add hl,de
	call pw_head
	jp nz,pw_leave
	jr pwr_memo
pwr_dummy:
	ld bc,0
pwr_memo:
	ld hl,(pv_mp)
	ld (hl),b
	inc hl
	ld (hl),c
	inc hl
	ld (pv_mp),hl
	ld a,(pv_rpage)
	out (7),a
	jr pwr_loop
pwr_done:
	ld a,2
	ld (pv_stage),a
	jp pw_run
