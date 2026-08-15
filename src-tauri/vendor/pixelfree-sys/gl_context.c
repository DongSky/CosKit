/*
 * Minimal offscreen OpenGL context for PixelFree on macOS (CGL).
 * PixelFree requires a current GL context before PF_NewPixelFree /
 * PF_processWithBuffer (the vendor demo uses a GLFW window; we use a
 * headless CGL context so no window is needed).
 */
#ifdef __APPLE__
#include <stddef.h>
#include <OpenGL/OpenGL.h>

static CGLContextObj g_pf_ctx = NULL;

int pf_gl_init(void) {
    if (g_pf_ctx) {
        return CGLSetCurrentContext(g_pf_ctx) == kCGLNoError ? 0 : -3;
    }
    CGLPixelFormatAttribute attrs[] = {
        kCGLPFAAccelerated,
        kCGLPFAOpenGLProfile,
        (CGLPixelFormatAttribute)kCGLOGLPVersion_Legacy,
        (CGLPixelFormatAttribute)0,
    };
    CGLPixelFormatObj pix = NULL;
    GLint npix = 0;
    if (CGLChoosePixelFormat(attrs, &pix, &npix) != kCGLNoError || !pix) {
        return -1;
    }
    CGLError err = CGLCreateContext(pix, NULL, &g_pf_ctx);
    CGLDestroyPixelFormat(pix);
    if (err != kCGLNoError || !g_pf_ctx) {
        g_pf_ctx = NULL;
        return -2;
    }
    return CGLSetCurrentContext(g_pf_ctx) == kCGLNoError ? 0 : -3;
}

void pf_gl_destroy(void) {
    if (g_pf_ctx) {
        CGLSetCurrentContext(NULL);
        CGLDestroyContext(g_pf_ctx);
        g_pf_ctx = NULL;
    }
}
#else
int pf_gl_init(void) { return -100; /* not implemented on this platform */ }
void pf_gl_destroy(void) {}
#endif
