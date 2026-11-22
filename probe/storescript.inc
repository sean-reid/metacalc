; The script loader of storecheck.z80, which includes this file unless SCRIPT_IN_RAM is defined.
; Copies MCCMD to SCRIPT, ended by a zero byte, or the default script when there is no MCCMD. An
; MCCMD in the archive runs nothing and logs SCRIPT_ARCHIVED, 2.
; Out: Z.
load_script:
	ld hl, OP1
	ld (hl), appVarObj
	inc hl
	ld (hl), 'M'
	inc hl
	ld (hl), 'C'
	inc hl
	ld (hl), 'C'
	inc hl
	ld (hl), 'M'
	inc hl
	ld (hl), 'D'
	inc hl
	ld (hl), 0
	bcall(_ChkFindSym)
	jr c, no_script
	ld a, b
	or a
	jr nz, archived_script
	ex de, hl
	ld c, (hl)
	inc hl
	ld b, (hl)
	inc hl
	ld de, SCRIPT_MAX - 1
	ld a, b
	cp d
	jr c, size_ok
	jr nz, size_cap
	ld a, c
	cp e
	jr c, size_ok
size_cap:
	ld b, d
	ld c, e
size_ok:
	ld de, SCRIPT
	ld a, b
	or c
	jr z, script_end
	ldir
script_end:
	xor a
	ld (de), a
	ret
archived_script:
	ld a, SCRIPT_ARCHIVED
	ld (S_RES), a
	ld a, 2
	ld (S_RES + 1), a
	ld (S_RES_LEN), a
	call log_result
	xor a
	ld (SCRIPT), a
	ret
no_script:
	ld hl, default_script
	ld de, SCRIPT
	ld bc, default_script_end - default_script
	ldir
	xor a
	ret

; The script of a bare run: a table of 10 entries holding 1 0 2 3 1 0 3 0 0 2, counted whole, to
; 6 and to 11, which is refused, and by two lists of ends, the second not rising and refused.
default_script:
	.db CMD_DROP
	.db CMD_OPEN, 026h, 039h, 0F4h, 0CBh, 10, 0
	.db CMD_SET, 0, 0, 1
	.db CMD_SET, 2, 0, 2
	.db CMD_SET, 3, 0, 3
	.db CMD_SET, 4, 0, 1
	.db CMD_SET, 6, 0, 3
	.db CMD_SET, 9, 0, 2
	.db CMD_GET, 9, 0
	.db CMD_COUNT_ENDS, 3, 3, 0, 6, 0, 10, 0
	.db CMD_COUNT
	.db CMD_COUNT_TO, 6, 0
	.db CMD_COUNT_TO, 11, 0
	.db CMD_COUNT_ENDS, 2, 6, 0, 3, 0
	.db CMD_DROP
	.db CMD_END
default_script_end:
