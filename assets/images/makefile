SVGS = $(wildcard *.svg)
PNGS = $(patsubst %.svg, %.png, $(SVGS))

all: $(PNGS)

clean:
	rm *.png

%.png : %.svg
	inkscape -e $@ -w 100 -h 100 $<
