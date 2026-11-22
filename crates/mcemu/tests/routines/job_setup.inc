; The setup the reference OS test gives the job probe, included by probe/jobcheck.z80 when JOB_WITH_SETUP
; is defined: an AppVar MCJOBT holding prep_data, archived or not, and a filler AppVar that leaves
; 20 bytes of RAM. The test writes prep_defs.inc (PREP_LEN, PREP_ARCHIVE, PREP_FILL) and
; prep_data.inc.
#include "prep_defs.inc"

job_setup:
#if PREP_LEN > 0
	call setup_name
	ld hl, PREP_LEN
	bcall(_CreateAppVar)
	inc de
	inc de
	ld hl, prep_data
	ld bc, PREP_LEN
	ldir
#if PREP_ARCHIVE
	call setup_name
	bcall(_Arc_Unarc)
#endif
#endif
#if PREP_FILL
	bcall(_MemChk)
	ld de, 15 + 20
	or a
	sbc hl, de
	ret c
	push hl
	ld hl, setup_filler
	ld de, OP1
	ld bc, 9
	ldir
	pop hl
	bcall(_CreateAppVar)
#endif
	ret

setup_name:
	ld hl, setup_table
	ld de, OP1
	ld bc, 9
	ldir
	ret

setup_table:
	.db appVarObj, "MCJOBT", 0, 0
setup_filler:
	.db appVarObj, "FILLER", 0, 0
#include "prep_data.inc"
