/*
 * GPUPixel C Wrapper
 *
 * Provides a C interface over the prebuilt GPUPixel v1.3.1 framework.
 * Pipeline: SourceRawData -> BeautyFaceFilter -> SinkRawData
 */

#pragma once

#include <stdint.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// Opaque handle for a complete beauty pipeline
typedef struct GPBeautyPipeline GPBeautyPipeline;

// Create/destroy a full pipeline (source -> beauty filter -> sink).
// Returns NULL on failure.
GPBeautyPipeline* gp_beauty_pipeline_create(void);
void gp_beauty_pipeline_destroy(GPBeautyPipeline* p);

// Beauty parameters (0.0 - 1.0 range; values are clamped internally)
void gp_beauty_set_white(GPBeautyPipeline* p, float value);
void gp_beauty_set_blur_alpha(GPBeautyPipeline* p, float value);
void gp_beauty_set_sharpen(GPBeautyPipeline* p, float value);

// Process one RGBA frame through the pipeline.
// Returns 0 on success. Output is valid until the next process call.
int gp_beauty_process_rgba(GPBeautyPipeline* p,
                           const uint8_t* rgba,
                           int width,
                           int height,
                           int stride_bytes);

// Get result of the last process call. Returns NULL if no frame processed.
const uint8_t* gp_beauty_get_rgba(GPBeautyPipeline* p, int* out_width, int* out_height);

#ifdef __cplusplus
}
#endif
