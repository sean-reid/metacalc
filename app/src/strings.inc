; The shell's constant strings, as glyph codes ended by 0, in one block for each screen. The canvas
; reads a string with its own page mapped, so a screen's draw copies its block to APP_STAGE with
; app_stage_block (text.inc) and hands the canvas the copy's addresses, the stg_ labels: a string's
; address in the block less the block's start, plus APP_STAGE_DATA. A block of more than
; APP_STAGE_DATA_SIZE bytes stops the assembly. A screen's strings that are built at run time are in
; APP_TEXT and are handed over where they are. The About screen's version is VERSION_MAJOR,
; VERSION_MINOR and VERSION_PATCH, which the Makefile takes from the workspace version.
#ifndef VERSION_MAJOR
 .error "VERSION_MAJOR, VERSION_MINOR and VERSION_PATCH name the version, as the Makefile passes them"
#endif
#macro VERSION_PART(n)
#if (n) >= 100
	.db UI_FONT_CH_30 + ((n) / 100)
#endif
#if (n) >= 10
	.db UI_FONT_CH_30 + (((n) / 10) % 10)
#endif
	.db UI_FONT_CH_30 + ((n) % 10)
#endmacro

libs_block:
ls_title:
	.db UI_FONT_CH_4C, UI_FONT_CH_69, UI_FONT_CH_62, UI_FONT_CH_72, UI_FONT_CH_61, UI_FONT_CH_72, UI_FONT_CH_69, UI_FONT_CH_65, UI_FONT_CH_73, 0
ls_right:
	.db UI_FONT_CH_6D, UI_FONT_CH_65, UI_FONT_CH_74, UI_FONT_CH_61, UI_FONT_CH_63, UI_FONT_CH_61, UI_FONT_CH_6C, UI_FONT_CH_63, 0
ls_proofs:
	.db UI_FONT_CH_4D, UI_FONT_CH_79, UI_FONT_CH_20, UI_FONT_CH_70, UI_FONT_CH_72, UI_FONT_CH_6F, UI_FONT_CH_6F, UI_FONT_CH_66, UI_FONT_CH_73, 0
ls_zero:
	.db UI_FONT_CH_30, 0
ls_none:
	.db 0
ls_open:
	.db UI_KEY_FONT_CH_4F, UI_KEY_FONT_CH_70, UI_KEY_FONT_CH_65, UI_KEY_FONT_CH_6E, 0
ls_run:
	.db UI_KEY_FONT_CH_52, UI_KEY_FONT_CH_75, UI_KEY_FONT_CH_6E, 0
ls_info:
	.db UI_KEY_FONT_CH_49, UI_KEY_FONT_CH_6E, UI_KEY_FONT_CH_66, UI_KEY_FONT_CH_6F, 0
ls_keys:
	.dw STG_LIBS_OPEN, STG_LIBS_RUN, STG_LIBS_INFO, STG_LIBS_NONE, STG_LIBS_NONE
libs_block_end:
STG_LIBS_TITLE	.equ APP_STAGE_DATA + (ls_title - libs_block)
STG_LIBS_RIGHT	.equ APP_STAGE_DATA + (ls_right - libs_block)
STG_LIBS_PROOFS	.equ APP_STAGE_DATA + (ls_proofs - libs_block)
STG_LIBS_ZERO	.equ APP_STAGE_DATA + (ls_zero - libs_block)
STG_LIBS_NONE	.equ APP_STAGE_DATA + (ls_none - libs_block)
STG_LIBS_OPEN	.equ APP_STAGE_DATA + (ls_open - libs_block)
STG_LIBS_RUN	.equ APP_STAGE_DATA + (ls_run - libs_block)
STG_LIBS_INFO	.equ APP_STAGE_DATA + (ls_info - libs_block)
STG_LIBS_KEYS	.equ APP_STAGE_DATA + (ls_keys - libs_block)
LIBS_BLOCK_SIZE	.equ libs_block_end - libs_block
#if LIBS_BLOCK_SIZE > APP_STAGE_DATA_SIZE
 .error "the Libraries screen's strings are longer than APP_STAGE_DATA_SIZE"
#endif

notice_block:
ns_verify:
	.db UI_FONT_CH_56, UI_FONT_CH_65, UI_FONT_CH_72, UI_FONT_CH_69, UI_FONT_CH_66, UI_FONT_CH_79, 0
ns_error:
	.db UI_FONT_CH_45, UI_FONT_CH_72, UI_FONT_CH_72, UI_FONT_CH_6F, UI_FONT_CH_72, 0
ns_unavailable:
	.db UI_FONT_CH_4E, UI_FONT_CH_6F, UI_FONT_CH_74, UI_FONT_CH_20, UI_FONT_CH_61, UI_FONT_CH_76, UI_FONT_CH_61, UI_FONT_CH_69, UI_FONT_CH_6C, UI_FONT_CH_61, UI_FONT_CH_62, UI_FONT_CH_6C, UI_FONT_CH_65, UI_FONT_CH_20, UI_FONT_CH_79, UI_FONT_CH_65, UI_FONT_CH_74, 0
ns_proofs:
	.db UI_FONT_CH_4D, UI_FONT_CH_79, UI_FONT_CH_20, UI_FONT_CH_70, UI_FONT_CH_72, UI_FONT_CH_6F, UI_FONT_CH_6F, UI_FONT_CH_66, UI_FONT_CH_73, 0
ns_pick:
	.db UI_FONT_CH_50, UI_FONT_CH_69, UI_FONT_CH_63, UI_FONT_CH_6B, UI_FONT_CH_20, UI_FONT_CH_61, UI_FONT_CH_20, UI_FONT_CH_6C, UI_FONT_CH_69, UI_FONT_CH_62, UI_FONT_CH_72, UI_FONT_CH_61, UI_FONT_CH_72, UI_FONT_CH_79, 0
ns_damaged:
	.db UI_FONT_CH_4C, UI_FONT_CH_69, UI_FONT_CH_62, UI_FONT_CH_72, UI_FONT_CH_61, UI_FONT_CH_72, UI_FONT_CH_79, UI_FONT_CH_20, UI_FONT_CH_69, UI_FONT_CH_73, UI_FONT_CH_20, UI_FONT_CH_64, UI_FONT_CH_61, UI_FONT_CH_6D, UI_FONT_CH_61, UI_FONT_CH_67, UI_FONT_CH_65, UI_FONT_CH_64, 0
ns_again:
	.db UI_FONT_CH_49, UI_FONT_CH_6E, UI_FONT_CH_73, UI_FONT_CH_74, UI_FONT_CH_61, UI_FONT_CH_6C, UI_FONT_CH_6C, UI_FONT_CH_20, UI_FONT_CH_69, UI_FONT_CH_74, UI_FONT_CH_20, UI_FONT_CH_61, UI_FONT_CH_67, UI_FONT_CH_61, UI_FONT_CH_69, UI_FONT_CH_6E, 0
ns_none:
	.db 0
ns_back:
	.db UI_KEY_FONT_CH_42, UI_KEY_FONT_CH_61, UI_KEY_FONT_CH_63, UI_KEY_FONT_CH_6B, 0
ns_keys:
	.dw STG_NOTICE_BACK, STG_NOTICE_NONE, STG_NOTICE_NONE, STG_NOTICE_NONE, STG_NOTICE_NONE
notice_block_end:
STG_NOTICE_VERIFY	.equ APP_STAGE_DATA + (ns_verify - notice_block)
STG_NOTICE_ERROR	.equ APP_STAGE_DATA + (ns_error - notice_block)
STG_NOTICE_PROOFS	.equ APP_STAGE_DATA + (ns_proofs - notice_block)
STG_NOTICE_PICK	.equ APP_STAGE_DATA + (ns_pick - notice_block)
STG_NOTICE_DAMAGED	.equ APP_STAGE_DATA + (ns_damaged - notice_block)
STG_NOTICE_AGAIN	.equ APP_STAGE_DATA + (ns_again - notice_block)
STG_NOTICE_UNAVAILABLE	.equ APP_STAGE_DATA + (ns_unavailable - notice_block)
STG_NOTICE_NONE	.equ APP_STAGE_DATA + (ns_none - notice_block)
STG_NOTICE_BACK	.equ APP_STAGE_DATA + (ns_back - notice_block)
STG_NOTICE_KEYS	.equ APP_STAGE_DATA + (ns_keys - notice_block)
NOTICE_BLOCK_SIZE	.equ notice_block_end - notice_block
#if NOTICE_BLOCK_SIZE > APP_STAGE_DATA_SIZE
 .error "the notices' strings are longer than APP_STAGE_DATA_SIZE"
#endif

about_block:
as_title:
	.db UI_FONT_CH_41, UI_FONT_CH_62, UI_FONT_CH_6F, UI_FONT_CH_75, UI_FONT_CH_74, 0
as_none:
	.db 0
as_version:
	.db UI_FONT_CH_6D, UI_FONT_CH_65, UI_FONT_CH_74, UI_FONT_CH_61, UI_FONT_CH_63, UI_FONT_CH_61, UI_FONT_CH_6C, UI_FONT_CH_63, UI_FONT_CH_20
	VERSION_PART(VERSION_MAJOR)
	.db UI_FONT_CH_2E
	VERSION_PART(VERSION_MINOR)
	.db UI_FONT_CH_2E
	VERSION_PART(VERSION_PATCH)
	.db 0
as_a:
	.db UI_FONT_CH_4D, UI_FONT_CH_65, UI_FONT_CH_74, UI_FONT_CH_61, UI_FONT_CH_6D, UI_FONT_CH_61, UI_FONT_CH_74, UI_FONT_CH_68, UI_FONT_CH_20, UI_FONT_CH_76, UI_FONT_CH_65, UI_FONT_CH_72, UI_FONT_CH_69, UI_FONT_CH_66, UI_FONT_CH_69, UI_FONT_CH_65, UI_FONT_CH_72, 0
as_b:
	.db UI_FONT_CH_61, UI_FONT_CH_6E, UI_FONT_CH_64, UI_FONT_CH_20, UI_FONT_CH_70, UI_FONT_CH_72, UI_FONT_CH_6F, UI_FONT_CH_6F, UI_FONT_CH_66, UI_FONT_CH_20, UI_FONT_CH_65, UI_FONT_CH_64, UI_FONT_CH_69, UI_FONT_CH_74, UI_FONT_CH_6F, UI_FONT_CH_72, 0
#ifdef LIBRARY
as_libs:
	.db UI_FONT_CH_31, UI_FONT_CH_20, UI_FONT_CH_6C, UI_FONT_CH_69, UI_FONT_CH_62, UI_FONT_CH_72, UI_FONT_CH_61, UI_FONT_CH_72, UI_FONT_CH_79, 0
#else
as_libs:
	.db UI_FONT_CH_30, UI_FONT_CH_20, UI_FONT_CH_6C, UI_FONT_CH_69, UI_FONT_CH_62, UI_FONT_CH_72, UI_FONT_CH_61, UI_FONT_CH_72, UI_FONT_CH_69, UI_FONT_CH_65, UI_FONT_CH_73, 0
#endif
about_block_end:
STG_ABOUT_TITLE	.equ APP_STAGE_DATA + (as_title - about_block)
STG_ABOUT_NONE	.equ APP_STAGE_DATA + (as_none - about_block)
STG_ABOUT_VERSION	.equ APP_STAGE_DATA + (as_version - about_block)
STG_ABOUT_A	.equ APP_STAGE_DATA + (as_a - about_block)
STG_ABOUT_B	.equ APP_STAGE_DATA + (as_b - about_block)
STG_ABOUT_LIBS	.equ APP_STAGE_DATA + (as_libs - about_block)
ABOUT_BLOCK_SIZE	.equ about_block_end - about_block
#if ABOUT_BLOCK_SIZE > APP_STAGE_DATA_SIZE
 .error "the About screen's strings are longer than APP_STAGE_DATA_SIZE"
#endif

list_block:
lq_none:
	.db UI_FONT_CH_6E, UI_FONT_CH_6F, UI_FONT_CH_6E, UI_FONT_CH_65, 0
lq_blank:
	.db 0
lq_open:
	.db UI_KEY_FONT_CH_4F, UI_KEY_FONT_CH_70, UI_KEY_FONT_CH_65, UI_KEY_FONT_CH_6E, 0
lq_find:
	.db UI_KEY_FONT_CH_46, UI_KEY_FONT_CH_69, UI_KEY_FONT_CH_6E, UI_KEY_FONT_CH_64, 0
lq_kind:
	.db UI_KEY_FONT_CH_4B, UI_KEY_FONT_CH_69, UI_KEY_FONT_CH_6E, UI_KEY_FONT_CH_64, 0
lq_try:
	.db UI_KEY_FONT_CH_54, UI_KEY_FONT_CH_72, UI_KEY_FONT_CH_79, 0
lq_sect:
	.db UI_KEY_FONT_CH_53, UI_KEY_FONT_CH_65, UI_KEY_FONT_CH_63, UI_KEY_FONT_CH_74, 0
lq_thm:
	.db UI_FONT_CH_3A, UI_FONT_CH_74, UI_FONT_CH_68, UI_FONT_CH_6D, 0
lq_ax:
	.db UI_FONT_CH_3A, UI_FONT_CH_61, UI_FONT_CH_78, 0
lq_unch:
	.db UI_FONT_CH_3A, UI_FONT_CH_3F, 0
lq_keys:
	.dw STG_LIST_OPEN, STG_LIST_FIND, STG_LIST_KIND, STG_LIST_TRY, STG_LIST_SECT
lq_keys_plain:
	.dw STG_LIST_OPEN, STG_LIST_FIND, STG_LIST_KIND, STG_LIST_TRY, STG_LIST_BLANK
list_block_end:
STG_LIST_NONE	.equ APP_STAGE_DATA + (lq_none - list_block)
STG_LIST_BLANK	.equ APP_STAGE_DATA + (lq_blank - list_block)
STG_LIST_OPEN	.equ APP_STAGE_DATA + (lq_open - list_block)
STG_LIST_FIND	.equ APP_STAGE_DATA + (lq_find - list_block)
STG_LIST_KIND	.equ APP_STAGE_DATA + (lq_kind - list_block)
STG_LIST_TRY	.equ APP_STAGE_DATA + (lq_try - list_block)
STG_LIST_SECT	.equ APP_STAGE_DATA + (lq_sect - list_block)
STG_LIST_THM	.equ APP_STAGE_DATA + (lq_thm - list_block)
STG_LIST_AX	.equ APP_STAGE_DATA + (lq_ax - list_block)
STG_LIST_UNCH	.equ APP_STAGE_DATA + (lq_unch - list_block)
STG_LIST_KEYS	.equ APP_STAGE_DATA + (lq_keys - list_block)
STG_LIST_KEYS_PLAIN	.equ APP_STAGE_DATA + (lq_keys_plain - list_block)
LIST_BLOCK_SIZE	.equ list_block_end - list_block
#if LIST_BLOCK_SIZE > APP_STAGE_DATA_SIZE
 .error "the list screen's strings are longer than APP_STAGE_DATA_SIZE"
#endif
lq_flat:
	.db UI_FONT_CH_66, UI_FONT_CH_6C, UI_FONT_CH_61, UI_FONT_CH_74, UI_FONT_CH_20, 0

theorem_block:
tz_fold:
	.db UI_KEY_FONT_CH_46, UI_KEY_FONT_CH_6F, UI_KEY_FONT_CH_6C, UI_KEY_FONT_CH_64, 0
tz_dist:
	.db UI_KEY_FONT_CH_44, UI_KEY_FONT_CH_69, UI_KEY_FONT_CH_73, UI_KEY_FONT_CH_74, 0
tz_why:
	.db UI_KEY_FONT_CH_57, UI_KEY_FONT_CH_68, UI_KEY_FONT_CH_79, 0
tz_try:
	.db UI_KEY_FONT_CH_54, UI_KEY_FONT_CH_72, UI_KEY_FONT_CH_79, 0
tz_none:
	.db 0
tz_line_1:
	.db UI_FONT_CH_54, UI_FONT_CH_68, UI_FONT_CH_69, UI_FONT_CH_73, UI_FONT_CH_20, UI_FONT_CH_74, UI_FONT_CH_68, UI_FONT_CH_65, UI_FONT_CH_6F, UI_FONT_CH_72, UI_FONT_CH_65, UI_FONT_CH_6D, UI_FONT_CH_20, UI_FONT_CH_69, UI_FONT_CH_73, UI_FONT_CH_20, UI_FONT_CH_74, UI_FONT_CH_6F, UI_FONT_CH_6F, 0
tz_line_2:
	.db UI_FONT_CH_6C, UI_FONT_CH_61, UI_FONT_CH_72, UI_FONT_CH_67, UI_FONT_CH_65, UI_FONT_CH_20, UI_FONT_CH_74, UI_FONT_CH_6F, UI_FONT_CH_20, UI_FONT_CH_73, UI_FONT_CH_68, UI_FONT_CH_6F, UI_FONT_CH_77, UI_FONT_CH_20, UI_FONT_CH_6F, UI_FONT_CH_6E, UI_FONT_CH_20, UI_FONT_CH_74, UI_FONT_CH_68, UI_FONT_CH_65, 0
tz_line_3:
	.db UI_FONT_CH_63, UI_FONT_CH_61, UI_FONT_CH_6C, UI_FONT_CH_63, UI_FONT_CH_75, UI_FONT_CH_6C, UI_FONT_CH_61, UI_FONT_CH_74, UI_FONT_CH_6F, UI_FONT_CH_72, 0
tz_keys:
	.dw STG_TH_FOLD, STG_TH_DIST, STG_TH_WHY, STG_TH_TRY, STG_TH_NONE
tz_keys_large:
	.dw STG_TH_NONE, STG_TH_DIST, STG_TH_NONE, STG_TH_TRY, STG_TH_NONE
theorem_block_end:
STG_TH_FOLD	.equ APP_STAGE_DATA + (tz_fold - theorem_block)
STG_TH_DIST	.equ APP_STAGE_DATA + (tz_dist - theorem_block)
STG_TH_WHY	.equ APP_STAGE_DATA + (tz_why - theorem_block)
STG_TH_TRY	.equ APP_STAGE_DATA + (tz_try - theorem_block)
STG_TH_NONE	.equ APP_STAGE_DATA + (tz_none - theorem_block)
STG_TH_LINE_1	.equ APP_STAGE_DATA + (tz_line_1 - theorem_block)
STG_TH_LINE_2	.equ APP_STAGE_DATA + (tz_line_2 - theorem_block)
STG_TH_LINE_3	.equ APP_STAGE_DATA + (tz_line_3 - theorem_block)
STG_TH_KEYS	.equ APP_STAGE_DATA + (tz_keys - theorem_block)
STG_TH_KEYS_LARGE	.equ APP_STAGE_DATA + (tz_keys_large - theorem_block)
THEOREM_BLOCK_SIZE	.equ theorem_block_end - theorem_block
#if THEOREM_BLOCK_SIZE > APP_STAGE_DATA_SIZE
 .error "the theorem screen's strings are longer than APP_STAGE_DATA_SIZE"
#endif

checking_block:
kx_checking:
	.db UI_FONT_CH_43, UI_FONT_CH_68, UI_FONT_CH_65, UI_FONT_CH_63, UI_FONT_CH_6B, UI_FONT_CH_69, UI_FONT_CH_6E, UI_FONT_CH_67, 0
kx_all:
	.db UI_FONT_CH_41, UI_FONT_CH_6C, UI_FONT_CH_6C, UI_FONT_CH_20, UI_FONT_CH_63, UI_FONT_CH_68, UI_FONT_CH_65, UI_FONT_CH_63, UI_FONT_CH_6B, UI_FONT_CH_65, UI_FONT_CH_64, 0
kx_reading:
	.db UI_FONT_CH_52, UI_FONT_CH_65, UI_FONT_CH_61, UI_FONT_CH_64, UI_FONT_CH_69, UI_FONT_CH_6E, UI_FONT_CH_67, UI_FONT_CH_20, UI_FONT_CH_6C, UI_FONT_CH_69, UI_FONT_CH_62, UI_FONT_CH_72, UI_FONT_CH_61, UI_FONT_CH_72, UI_FONT_CH_79, 0
kx_pauses:
	.db UI_FONT_CH_4F, UI_FONT_CH_4E, UI_FONT_CH_20, UI_FONT_CH_70, UI_FONT_CH_61, UI_FONT_CH_75, UI_FONT_CH_73, UI_FONT_CH_65, UI_FONT_CH_73, 0
kx_none:
	.db 0
kx_stop:
	.db UI_KEY_FONT_CH_53, UI_KEY_FONT_CH_74, UI_KEY_FONT_CH_6F, UI_KEY_FONT_CH_70, 0
kx_done:
	.db UI_KEY_FONT_CH_44, UI_KEY_FONT_CH_6F, UI_KEY_FONT_CH_6E, UI_KEY_FONT_CH_65, 0
kx_keys_stop:
	.dw STG_CK_STOP, STG_CK_NONE, STG_CK_NONE, STG_CK_NONE, STG_CK_NONE
kx_keys_done:
	.dw STG_CK_DONE, STG_CK_NONE, STG_CK_NONE, STG_CK_NONE, STG_CK_NONE
checking_block_end:
STG_CK_CHECKING	.equ APP_STAGE_DATA + (kx_checking - checking_block)
STG_CK_ALL	.equ APP_STAGE_DATA + (kx_all - checking_block)
STG_CK_READING	.equ APP_STAGE_DATA + (kx_reading - checking_block)
STG_CK_PAUSES	.equ APP_STAGE_DATA + (kx_pauses - checking_block)
STG_CK_NONE	.equ APP_STAGE_DATA + (kx_none - checking_block)
STG_CK_STOP	.equ APP_STAGE_DATA + (kx_stop - checking_block)
STG_CK_DONE	.equ APP_STAGE_DATA + (kx_done - checking_block)
STG_CK_KEYS_STOP	.equ APP_STAGE_DATA + (kx_keys_stop - checking_block)
STG_CK_KEYS_DONE	.equ APP_STAGE_DATA + (kx_keys_done - checking_block)
CHECKING_BLOCK_SIZE	.equ checking_block_end - checking_block
#if CHECKING_BLOCK_SIZE > APP_STAGE_DATA_SIZE
 .error "the checking screen's strings are longer than APP_STAGE_DATA_SIZE"
#endif
ck_failed:
	.db UI_FONT_CH_20, UI_FONT_CH_66, UI_FONT_CH_61, UI_FONT_CH_69, UI_FONT_CH_6C, UI_FONT_CH_65, UI_FONT_CH_64, 0
