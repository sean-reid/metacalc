#include "ram_map.inc"
#ifndef LIB_SPARE_PAGE
LIB_SPARE_PAGE      .equ 3
#endif
#if LIB_SPARE_PAGE > 7
 .error "LIB_SPARE_PAGE must be a RAM page, 0 to 7"
#endif
#if LIB_BOUNCE < $C000
 .error "LIB_BOUNCE must be in bank C"
#endif
#if (LIB_BOUNCE + LIB_BOUNCE_SIZE) > $10000
 .error "LIB_BOUNCE must leave 256 bytes below $10000"
#endif
#ifndef LIB_BOUNCE_ANYWHERE
LIB_BOUNCE_IN_LOW   .equ (LIB_BOUNCE >= KERNEL_RAM_LOW) && ((LIB_BOUNCE + LIB_BOUNCE_SIZE) <= (KERNEL_RAM_LOW + LOW_SIZE))
LIB_BOUNCE_IN_HIGH  .equ (LIB_BOUNCE >= KERNEL_RAM_HIGH) && ((LIB_BOUNCE + LIB_BOUNCE_SIZE) <= (KERNEL_RAM_HIGH + HIGH_SIZE))
#if (LIB_BOUNCE_IN_LOW + LIB_BOUNCE_IN_HIGH) == 0
 .error "LIB_BOUNCE must lie inside the kernel regions, KERNEL_RAM_LOW or KERNEL_RAM_HIGH"
#endif
#endif
#if LIB_STATE < $8000
 .error "LIB_STATE must be in bank B"
#endif
LIB_INDEX           .equ LIB_STATE+$A8
#if LIB_STATE + LIB_STATE_SIZE > $C000
 .error "LIB_STATE and its index must end below $C000"
#endif
OPEN_MAGIC          .equ $4C42

#macro LR_VAR(name, offset, size)
RAM_VAR(name, LIB_STATE, offset, size, LIB_STATE_SIZE, "the reader's state runs past LIB_STATE_SIZE in ram_map.inc")
#endmacro

LR_VAR(lr_dest, $00, 2)
LR_VAR(lr_off, $02, 3)
LR_VAR(lr_old5, $05, 1)
LR_VAR(lr_iff, $06, 1)
LR_VAR(lr_page1, $07, 1)
LR_VAR(lr_n2, $08, 1)
LR_VAR(lr_src, $09, 2)
LR_VAR(lr_n1, $0B, 5)

LR_VAR(lr_open, $10, 2)
LR_VAR(lr_page, $12, 1)
LR_VAR(lr_length, $13, 3)
LR_VAR(lr_consts, $16, 1)
LR_VAR(lr_nsyms, $17, 1)
LR_VAR(lr_count, $18, 2)
LR_VAR(lr_dir, $1A, 6)

LR_VAR(lr_dn, $20, 2)
LR_VAR(lr_drec, $22, 4)
LR_VAR(lr_dkind, $26, 1)
LR_VAR(lr_dllen, $27, 1)
LR_VAR(lr_dnhyps, $28, 1)
LR_VAR(lr_dhyps, $29, 3)
LR_VAR(lr_dnpairs, $2C, 2)
LR_VAR(lr_dpairs, $2E, 3)
LR_VAR(lr_dconc, $31, 3)
LR_VAR(lr_dclen, $34, 1)
LR_VAR(lr_hk, $35, 1)
LR_VAR(lr_hoff, $36, 10)

LR_VAR(lr_bufp, $40, 2)
LR_VAR(lr_scratch, $42, 2)
LR_VAR(lr_hdr, $44, 4)
LR_VAR(lr_tr, $48, 4)
LR_VAR(lr_sr, $4C, 2)
LR_VAR(lr_sid, $4E, 1)
LR_VAR(lr_sbase, $4F, 1)
LR_VAR(lr_want, $50, 1)
LR_VAR(lr_id, $51, 1)
LR_VAR(lr_ent, $52, 3)
LR_VAR(lr_len, $55, 1)
LR_VAR(lr_tc, $56, 1)
LR_VAR(lr_n, $57, 2)
LR_VAR(lr_k, $59, 1)
LR_VAR(lr_cnt, $5A, 1)
LR_VAR(lr_wbase, $5B, 3)
LR_VAR(lr_wlen, $5E, 1)
LR_VAR(lr_wp, $5F, 1)
LR_VAR(lr_win, $60, 32)
LIB_WINDOW          .equ 32
LR_VAR(lr_dcbase, $80, 2)
LR_VAR(lr_q, $82, 2)
LR_VAR(lr_dc, $84, 32)
LR_VAR(lr_cn, $A4, 4)

LR_VAR(ls_have, $2A8, 1)
LR_VAR(ls_ti, $2A9, 9)
LR_VAR(ls_gl, $2B2, 9)
LR_VAR(ls_tr, $2BB, 9)
LR_VAR(ls_ti_n, $2C4, 2)
LR_VAR(ls_gl_fp, $2C6, 2)
LR_VAR(ls_tr_n, $2C8, 2)
LR_VAR(ls_ts, $2CA, 3)
LR_VAR(ls_te, $2CD, 3)
LS_TITLES           .equ 1
LS_GLYPHS           .equ 2
LS_TREES            .equ 4
LS_FONT             .equ 8
LS_FIRST            .equ 6

ls_lo               .equ lr_n
ls_hi               .equ lr_sr
ls_mid              .equ lr_q
ls_k                .equ lr_cn

LIB_OK              .equ 0
LIB_ERR_RANGE       .equ 1
LIB_ERR_ARG         .equ 2
LIB_ERR_BAD         .equ 3
LIB_ERR_CLOSED      .equ 4
LIB_ERR_INDEX       .equ 5
LIB_ERR_TRAILER     .equ 6
LIB_ERR_TABLE       .equ 7
LIB_ERR_SECTION     .equ 8
LIB_ERR_NONE        .equ 9
LIB_ERR_GLYPHS      .equ 10
