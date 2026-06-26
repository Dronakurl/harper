# Harper Language Statistics and Analysis

# Import language-specific recipes
import "harper-core/src/language/justfile"

# Language statistics - analyze all languages
language-stats:
	@echo "📊 Harper Language Statistics"
	@echo "==============================="
	
	# Create output directory
	@mkdir -p .stats
	
	# Analyze each language
	@for lang in english german portuguese; do \
	    echo "Analyzing $$lang language..." && \
	    cargo run --quiet --bin harper-lang-stats -- $$lang > .stats/$$lang-stats.txt 2>/dev/null || \
	    echo "⚠️  Could not analyze $$lang"; \
	    echo; \
	done
	
	@echo "✅ Statistics generated in .stats/ directory"
	@echo "=========================================="

# German coverage analysis
german-coverage:
	@echo "🔍 German Coverage Analysis"
	@echo "==========================="
	@python3 scripts/german_coverage.py
	@echo "==========================="

# Clean statistics
clean-stats:
	@rm -rf .stats
	@echo "✅ Statistics cleaned"

# Show German statistics
german-stats:
	@echo "📊 German Language Statistics"
	@echo "=============================="
	@cargo run --quiet --bin harper-lang-stats -- german
	@echo "=============================="

# Validate all dictionaries
validate-dicts:
	@echo "🔍 Validating Dictionaries"
	@echo "=========================="
	@for lang in english german portuguese; do \
	    echo "Validating $$lang dictionary..." && \
	    cargo run --quiet --bin harper-dict-validator -- $$lang || \
	    echo "❌ $$lang dictionary has issues"; \
	    echo; \
	done
	@echo "=========================="

# Full language analysis
full-analysis:
	@just language-stats
	@just german-coverage
	@just validate-dicts