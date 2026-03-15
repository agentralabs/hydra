package frame

// Breakpoint determines the responsive layout based on terminal width.
type Breakpoint int

const (
	BreakWide   Breakpoint = iota // >= 120 cols: full layout with logo
	BreakMedium                   // 60-119 cols: two columns, no logo
	BreakNarrow                   // < 60 cols: single column essentials
)

// DetectBreakpoint returns the current breakpoint for the given width.
func DetectBreakpoint(width int) Breakpoint {
	if width >= 120 {
		return BreakWide
	} else if width >= 60 {
		return BreakMedium
	}
	return BreakNarrow
}

// FrameHeight returns the frame height for the given breakpoint.
func FrameHeight(bp Breakpoint) int {
	switch bp {
	case BreakWide:
		return 14
	case BreakMedium:
		return 8
	case BreakNarrow:
		return 4
	}
	return 8
}
