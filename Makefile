.PHONY: test-domineering-search
test-domineering-search:
	$(eval OUT_FILE := $(shell mktemp))
	cargo run -p cgt-cli --		\
		domineering search	\
		--width 4		\
		--height 5		\
		--progress-interval 1	\
		--output-path $(OUT_FILE)
	sort -o $(OUT_FILE) $(OUT_FILE)
	diff $(OUT_FILE) ./test-data/out4x5-sorted.json
