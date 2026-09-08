MAKEFILE_DIR := $(patsubst %/,%,$(dir $(abspath $(lastword $(MAKEFILE_LIST)))))
UV := uv
RUN := $(UV) run --directory $(MAKEFILE_DIR)
SOURCES := src tests

.DEFAULT_GOAL := check
.PHONY: install fmt lint types test worker-check check cov install-hooks clean

install:
	cd $(MAKEFILE_DIR) && $(UV) sync --all-extras --dev
	cd $(MAKEFILE_DIR) && npm install

fmt:
	$(RUN) ruff format $(SOURCES)
	$(RUN) ruff check --fix $(SOURCES)

lint:
	$(RUN) ruff format --check $(SOURCES)
	$(RUN) ruff check $(SOURCES)

types:
	$(RUN) mypy $(SOURCES)

test:
	$(RUN) pytest

worker-check:
	cd $(MAKEFILE_DIR) && npm run check

check: lint types test worker-check

cov:
	$(RUN) pytest --cov=src --cov-report=term-missing

install-hooks:
	@bash $(MAKEFILE_DIR)/scripts/install_hooks.sh

clean:
	cd $(MAKEFILE_DIR) && rm -rf .mypy_cache .ruff_cache .pytest_cache .coverage htmlcov dist build
	cd $(MAKEFILE_DIR) && find . -name '__pycache__' -type d -prune -exec rm -rf {} +

