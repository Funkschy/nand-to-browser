install:
	latexmk -pdf thesis.tex
	latexmk -c thesis.tex
	pdftk A=thesis.pdf B=fig/erklärung.pdf cat A1-2 B1 A4-end output signed.pdf
	mv signed.pdf thesis.pdf

watch:
	latexmk -pvc thesis.tex

clean:
	latexmk -C thesis.tex

bibtool:
	bibtool -R -r keep_bibtex -r field -r improve -r month -s references.bib -o references.bib
