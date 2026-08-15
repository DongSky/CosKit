/*
 * GPUPixel C Wrapper Implementation (prebuilt v1.3.1 API)
 */

#include "wrapper.h"

#include "gpupixel/filter/beauty_face_filter.h"
#include "gpupixel/sink/sink_raw_data.h"
#include "gpupixel/source/source_raw_data.h"

#include <memory>

using namespace gpupixel;

struct GPBeautyPipeline {
    std::shared_ptr<SourceRawData> source;
    std::shared_ptr<BeautyFaceFilter> filter;
    std::shared_ptr<SinkRawData> sink;
    bool has_frame = false;
};

extern "C" {

GPBeautyPipeline* gp_beauty_pipeline_create(void) {
    auto* p = new (std::nothrow) GPBeautyPipeline();
    if (!p) {
        return nullptr;
    }

    p->source = SourceRawData::Create();
    p->filter = BeautyFaceFilter::Create();
    p->sink = SinkRawData::Create();

    if (!p->source || !p->filter || !p->sink) {
        delete p;
        return nullptr;
    }

    p->source->AddSink(p->filter);
    p->filter->AddSink(p->sink);
    return p;
}

void gp_beauty_pipeline_destroy(GPBeautyPipeline* p) {
    delete p;
}

void gp_beauty_set_white(GPBeautyPipeline* p, float value) {
    if (p && p->filter) {
        p->filter->SetWhite(value);
    }
}

void gp_beauty_set_blur_alpha(GPBeautyPipeline* p, float value) {
    if (p && p->filter) {
        p->filter->SetBlurAlpha(value);
    }
}

void gp_beauty_set_sharpen(GPBeautyPipeline* p, float value) {
    if (p && p->filter) {
        p->filter->SetSharpen(value);
    }
}

int gp_beauty_process_rgba(GPBeautyPipeline* p,
                           const uint8_t* rgba,
                           int width,
                           int height,
                           int stride_bytes) {
    if (!p || !p->source || !rgba || width <= 0 || height <= 0) {
        return -1;
    }
    p->source->ProcessData(rgba, width, height, stride_bytes,
                           GPUPIXEL_FRAME_TYPE_RGBA);
    p->has_frame = true;
    return 0;
}

const uint8_t* gp_beauty_get_rgba(GPBeautyPipeline* p, int* out_width, int* out_height) {
    if (!p || !p->sink || !p->has_frame) {
        return nullptr;
    }
    if (out_width) {
        *out_width = p->sink->GetWidth();
    }
    if (out_height) {
        *out_height = p->sink->GetHeight();
    }
    return p->sink->GetRgbaBuffer();
}

}  // extern "C"
