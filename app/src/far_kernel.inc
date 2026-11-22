; The kernel's public routines as the entry page calls them: one stub each, in the data page's
; far entry (pages.inc).
FAR_DATA_ENTRY(kernel_init, far_kernel_init)
FAR_DATA_ENTRY(kernel_reset, far_kernel_reset)
FAR_DATA_ENTRY(kernel_verify, far_kernel_verify)
FAR_DATA_ENTRY(kernel_crc_begin, far_kernel_crc_begin)
FAR_DATA_ENTRY(kernel_crc_chunk, far_kernel_crc_chunk)
FAR_DATA_ENTRY(kernel_refs, far_kernel_refs)
