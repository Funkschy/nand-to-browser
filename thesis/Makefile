install:
	latexmk -pdf thesis.tex
	latexmk -c thesis.tex

watch:
	latexmk -pvc thesis.tex

clean:
	latexmk -C thesis.tex

bibtool:
	bibtool -R -r keep_bibtex -r field -r improve -r month -s references.bib -o references.bib
