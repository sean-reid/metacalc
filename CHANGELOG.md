# Changelog

## 0.1.0 (2026-11-19)


### Features

* add mcpack workspace skeleton ([505d2d8](https://github.com/sean-reid/metacalc/commit/505d2d82cc61856640ce3be7458a71c29317ec1e))
* **app:** keep which assertions are checked in an AppVar ([#36](https://github.com/sean-reid/metacalc/issues/36)) ([ef6926d](https://github.com/sean-reid/metacalc/commit/ef6926d714b9bde6c93c5fb9498536aa819acf6d))
* **app:** store the library in the app's own pages ([#25](https://github.com/sean-reid/metacalc/issues/25)) ([88d1808](https://github.com/sean-reid/metacalc/commit/88d18083afd98be6022e78069a0fbb7fe5be9cd6))
* **docs:** give the site a calculator favicon ([#91](https://github.com/sean-reid/metacalc/issues/91)) ([4f7d000](https://github.com/sean-reid/metacalc/commit/4f7d000d745f70b794f32ccaaaf2a6b48235d89e))
* install metacalc with one curl command and attach the sender to each release ([#107](https://github.com/sean-reid/metacalc/issues/107)) ([066339f](https://github.com/sean-reid/metacalc/commit/066339f2af905c825a7b3e4f78e0fce67c8d1fb4))
* **kernel:** bound the disjoint-variable check and poll the ON key ([#26](https://github.com/sean-reid/metacalc/issues/26)) ([b7317c6](https://github.com/sean-reid/metacalc/commit/b7317c6030d951b3f6af87c08c305ee3b358aa7f))
* **kernel:** check a packed library in Z80 assembly ([#16](https://github.com/sean-reid/metacalc/issues/16)) ([1d6b6d7](https://github.com/sean-reid/metacalc/commit/1d6b6d7d8e1d486f532503582ce3511789289d26))
* **kernel:** check the library CRC-32 in 16 KB steps ([#20](https://github.com/sean-reid/metacalc/issues/20)) ([b14a0bb](https://github.com/sean-reid/metacalc/commit/b14a0bbb37a6396f941625031f7278cbff8a539c))
* **kernel:** keep kernel RAM on the spare page ([#31](https://github.com/sean-reid/metacalc/issues/31)) ([2603654](https://github.com/sean-reid/metacalc/commit/2603654e7b85ee57568df050796339d003e6c759))
* **kernel:** refuse reference lists that are not in first-use order ([#32](https://github.com/sean-reid/metacalc/issues/32)) ([9bdcc84](https://github.com/sean-reid/metacalc/commit/9bdcc849a7a88dfed2a6d4136bf87624768abcee))
* **lib:** add a Z80 reader for the packed library ([#86](https://github.com/sean-reid/metacalc/issues/86)) ([da1de2c](https://github.com/sean-reid/metacalc/commit/da1de2c4ef75157555d0763ada18b1fd0c1fa910))
* **lib:** read section titles, the glyph map and parse trees on the Z80 ([#108](https://github.com/sean-reid/metacalc/issues/108)) ([0533a8e](https://github.com/sean-reid/metacalc/commit/0533a8e98074362d562b45e9b5dbdfa966b03a64))
* **mcedit:** add the proof editor model ([#19](https://github.com/sean-reid/metacalc/issues/19)) ([d475d35](https://github.com/sean-reid/metacalc/commit/d475d359260a7db97efd97bfee0c9571b559e9d8))
* **mcemu:** run apps from their entry point on the stub OS ([#28](https://github.com/sean-reid/metacalc/issues/28)) ([0ec05e7](https://github.com/sean-reid/metacalc/commit/0ec05e72623b30566352c4e30fa550158e73117c))
* **mcemu:** run calculator code on the TilEm core in tests ([#11](https://github.com/sean-reid/metacalc/issues/11)) ([a4428a4](https://github.com/sean-reid/metacalc/commit/a4428a4e7df5656c5c761111895b97ca012dee14))
* **mcglyph:** add bitmap font and token typesetting ([#13](https://github.com/sean-reid/metacalc/issues/13)) ([57d715b](https://github.com/sean-reid/metacalc/commit/57d715b80ae247d997d83d8edb4816dff2f212d4))
* **mcpack:** build libraries from manifests and publish their reference pages ([#12](https://github.com/sean-reid/metacalc/issues/12)) ([19ac1e5](https://github.com/sean-reid/metacalc/commit/19ac1e5a3987d8b4cd623437db949d5b8e8859a6))
* **mcpack:** carry section titles, glyph codes and parse trees in a library ([#88](https://github.com/sean-reid/metacalc/issues/88)) ([b520d54](https://github.com/sean-reid/metacalc/commit/b520d54634109434330e18f91abe08ababfe92b8))
* **mcpack:** export a packed library as a Metamath database ([#14](https://github.com/sean-reid/metacalc/issues/14)) ([9a861ae](https://github.com/sean-reid/metacalc/commit/9a861ae088955fb97b85e59b32102526a5fc668d))
* **mcpack:** measure the dependency closure of chosen theorems ([#6](https://github.com/sean-reid/metacalc/issues/6)) ([ac08066](https://github.com/sean-reid/metacalc/commit/ac0806689c2663a0f1862e3ee0efd23e1e4da989))
* **mcpack:** pack a closure into a binary library and check it with a kernel model ([#9](https://github.com/sean-reid/metacalc/issues/9)) ([1577a9d](https://github.com/sean-reid/metacalc/commit/1577a9d60b90c4439351e0a8d184c4405ad36aff))
* **mcpack:** parse and verify Metamath databases ([#5](https://github.com/sean-reid/metacalc/issues/5)) ([e0b82fc](https://github.com/sean-reid/metacalc/commit/e0b82fcc88d8b9dd9c350a233c2e801c6abdf63f))
* **mcpack:** parse statements with the database's syntax axioms ([#7](https://github.com/sean-reid/metacalc/issues/7)) ([179723d](https://github.com/sean-reid/metacalc/commit/179723dffcef202941018b5baa91a55846b5abdc))
* **mcsend:** send a Flash App to the calculator over USB ([#95](https://github.com/sean-reid/metacalc/issues/95)) ([a3b2e2c](https://github.com/sean-reid/metacalc/commit/a3b2e2caeb1ddaff2aee3cbaac77ee680c69fd4f))
* **mcui:** add a host model of the calculator screens ([#21](https://github.com/sean-reid/metacalc/issues/21)) ([2305518](https://github.com/sean-reid/metacalc/commit/2305518309d92a6f7e426e5d03221a49b0cd1a5e))
* **mcui:** add the proving and checking screens ([#27](https://github.com/sean-reid/metacalc/issues/27)) ([0e15aa6](https://github.com/sean-reid/metacalc/commit/0e15aa67bb593a1a56fd64650287f2e47dd99e53))
* **mcui:** jump to a section from the theorem list ([#109](https://github.com/sean-reid/metacalc/issues/109)) ([884abb6](https://github.com/sean-reid/metacalc/commit/884abb63692550ed6bda4093701fa4e5f0de9489))
* **mcui:** set the soft keys in a narrow font ([#29](https://github.com/sean-reid/metacalc/issues/29)) ([115b831](https://github.com/sean-reid/metacalc/commit/115b83173475b6bfe7269547c7859b6a1838deb2))
* **probe:** add a RAM page probe app for the TI-84 Plus SE ([#18](https://github.com/sean-reid/metacalc/issues/18)) ([27e6730](https://github.com/sean-reid/metacalc/commit/27e673096292a02a2cb07d096cfd10e7c7004f71))
* **probe:** add an app that checks the spare page regions survive TI-OS ([#24](https://github.com/sean-reid/metacalc/issues/24)) ([9f0abd9](https://github.com/sean-reid/metacalc/commit/9f0abd9ea5ca2f9f0a4e87bb4dfa8ebc68f98ea1))
* **ui:** add a Z80 drawing layer that matches the model pixel for pixel ([#30](https://github.com/sean-reid/metacalc/issues/30)) ([92f87da](https://github.com/sean-reid/metacalc/commit/92f87dae716b6bd6be06fa2f46f8bd07ca26dff5))


### Bug Fixes

* **build:** fail the build when spasm reports an error ([#37](https://github.com/sean-reid/metacalc/issues/37)) ([9827175](https://github.com/sean-reid/metacalc/commit/9827175c40344e2e0d2dd5cf6b8bd24ab76323a7))
* **ci:** pin actions and make the docs and release checks fail when they should ([#89](https://github.com/sean-reid/metacalc/issues/89)) ([58e1b9c](https://github.com/sean-reid/metacalc/commit/58e1b9c85b6d43bd7efb123e4225b28b02dbafc9))
* **glyph:** draw F/_ as a turned F and separate both underlines ([#87](https://github.com/sean-reid/metacalc/issues/87)) ([082821c](https://github.com/sean-reid/metacalc/commit/082821ca55035b216ccd4425aa691de5a8126743))
* **kernel:** check symbol names at init and refuse records the exporter refuses ([#102](https://github.com/sean-reid/metacalc/issues/102)) ([09f4797](https://github.com/sean-reid/metacalc/commit/09f479764b4e1dbb0d1e8fbf9aabc2823fbd5237))
* **kernel:** handle a saved buffer ending at 10000h and guard layout inputs ([#83](https://github.com/sean-reid/metacalc/issues/83)) ([73c5a38](https://github.com/sean-reid/metacalc/commit/73c5a386b3ad71e8d76490523e06a8106bd78652))
* **mcedit:** owe the disjoint pairs of syntax axioms in terms the proof builds ([#106](https://github.com/sean-reid/metacalc/issues/106)) ([d6971c3](https://github.com/sean-reid/metacalc/commit/d6971c3fef94c6f7cb92cd06b1245d90d194d35f))
* **mcedit:** report repeated violations, bound terms, and tie a save to its library ([#92](https://github.com/sean-reid/metacalc/issues/92)) ([6cf753a](https://github.com/sean-reid/metacalc/commit/6cf753a237767773487704f09db6e04bcbe282b4))
* **mcemu:** make the harness and stub OS as strict as the calculator ([#101](https://github.com/sean-reid/metacalc/issues/101)) ([8894a48](https://github.com/sean-reid/metacalc/commit/8894a4814612610fb61d4b3481a3ee1d84e6682c))
* **mcemu:** report no key from _GetCSC while two are down ([#35](https://github.com/sean-reid/metacalc/issues/35)) ([e1961d0](https://github.com/sean-reid/metacalc/commit/e1961d0e887c33ca63bfc0187f51c29581047394))
* **mcemu:** stop only at a real HALT and return at once on a zero limit ([#85](https://github.com/sean-reid/metacalc/issues/85)) ([dc2d474](https://github.com/sean-reid/metacalc/commit/dc2d47416ba6777438f7e32ed0c35e708a1065fe))
* **mcpack:** bound check, refuse overlapping records, and read file inclusion ([#104](https://github.com/sean-reid/metacalc/issues/104)) ([5d497e4](https://github.com/sean-reid/metacalc/commit/5d497e44556d9a9fcaa19ca1ecca29475ac6c36a))
* **mcpack:** declare every variable's float once in an export ([#98](https://github.com/sean-reid/metacalc/issues/98)) ([8bb838d](https://github.com/sean-reid/metacalc/commit/8bb838d259c0e2eb78fc244a34c403fb7ab1b472))
* **mcpack:** refuse a cut that drops a needed syntax axiom, and pin eight untested rules ([#113](https://github.com/sean-reid/metacalc/issues/113)) ([d2dad0b](https://github.com/sean-reid/metacalc/commit/d2dad0b85b7206f68bd9f7ff4b0a142f36d8f5e1))
* **mcsend:** open the link again after each end of transmission ([#105](https://github.com/sean-reid/metacalc/issues/105)) ([134a286](https://github.com/sean-reid/metacalc/commit/134a286ff9b87d09ee48e916fb231cf5c9a8751e))
* **mcui:** scroll tall goals and flat statements, bound the stack, and distrust a damaged library ([#100](https://github.com/sean-reid/metacalc/issues/100)) ([ab39433](https://github.com/sean-reid/metacalc/commit/ab3943384ffe49436a5e5eaa635336a74392f5e0))
* **probe:** catch ON between polls and leave the OS as the app found it ([#111](https://github.com/sean-reid/metacalc/issues/111)) ([376ba20](https://github.com/sean-reid/metacalc/commit/376ba2099f211f0fdf781e4c137452b904b84a64))
* **release:** start from the initial version so the first release is 0.1.0 ([#93](https://github.com/sean-reid/metacalc/issues/93)) ([f92d057](https://github.com/sean-reid/metacalc/commit/f92d057dbe7e4f202b41ae23cd96785ca4565466))
* small defects in Find, key labels, glyphs, typecode and notation ([#82](https://github.com/sean-reid/metacalc/issues/82)) ([be9acca](https://github.com/sean-reid/metacalc/commit/be9acca0e948ac1426e3c67aa3c29715ee96b5b6))
* **ui:** leave a clear pixel row between the content and the soft keys ([#114](https://github.com/sean-reid/metacalc/issues/114)) ([e1009d8](https://github.com/sean-reid/metacalc/commit/e1009d8a2cca2aaf2884e20c718b4bfd9ceb481e))
* **ui:** leave the top pixel row of a highlighted row clear ([#112](https://github.com/sean-reid/metacalc/issues/112)) ([b8ee14f](https://github.com/sean-reid/metacalc/commit/b8ee14fe72dc236ad6e9cfd1520ecf04c0091371))


### Performance

* **ui:** cut the typical frame from 192,758 to 143,775 T-states ([#33](https://github.com/sean-reid/metacalc/issues/33)) ([a1f6246](https://github.com/sean-reid/metacalc/commit/a1f62460f909153746fd2e9a1d2814a838ac90a1))
