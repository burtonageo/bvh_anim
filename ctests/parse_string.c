#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <errno.h>

#include "bvh_anim/bvh_anim.h"

int main(int argc, const char* argv[]) {
    const char* fname = NULL;

    if (argc <= 1) {
        fname = "./data/test_mocapbank.bvh";
    } else {
        fname = argv[1];
    }

    FILE* bvh_file = fopen(fname, "r");

    if (bvh_file == NULL) {
        fprintf(stderr, "Could not open bvh file '%s'\n", fname);
        return EXIT_FAILURE;
    }

    struct bvh_BvhFile bvh = {0};
    const int result = bvh_file_read(bvh_file, &bvh);

    const int close_result = fclose(bvh_file);

    if (result == 0) {
        fprintf(stderr, "Could not parse bvh file test_mocapbank.bvh\n");
        return EXIT_FAILURE;
    }

    if (close_result != 0) {
        fprintf(stderr, "Could not close bvh file test_mocapbank.bvh\n");
        fprintf(stderr, "errno = %d, and error = %s\n", errno, strerror(errno));
        return EXIT_FAILURE;
    }

    const size_t num_joints = bvh_file_get_num_joints(&bvh);
    const struct bvh_Joint* joints = bvh_file_get_joints(&bvh);

    printf("Num joints = %zu\n", num_joints);

    for (size_t i = 0; i < num_joints; i++) {
        const struct bvh_Joint* joint = &joints[i];

        const size_t depth = bvh_joint_get_depth(joint) * 2;
        char* indent = (char*)calloc(depth + 1, sizeof(char));
        memset(indent, ' ', depth);

        printf("%sJoint name = \"%s\"\n", indent, bvh_joint_get_name(joint));
        printf("%sJoint depth = %zu\n", indent, depth);

        const struct bvh_Offset offset = bvh_joint_get_offset(joint);
        printf(
            "%sJoint offset = (%f, %f, %f)\n",
            indent,
            offset.offset_x,
            offset.offset_y,
            offset.offset_z);

        const size_t num_channels = bvh_joint_get_num_channels(joint);
        const struct bvh_Channel* channels = bvh_joint_get_channels(joint);

        if (num_channels > 0) {
            printf("%sChannels = [", indent);
        }

        for (size_t curr_channel = 0; curr_channel < num_channels; curr_channel++) {
            const struct bvh_Channel* channel = &channels[curr_channel];
            printf("%zu: ", channel->channel_index);
            switch (channel->channel_type) {
                case X_POSITION: printf("Xposition"); break;
                case Y_POSITION: printf("Yposition"); break;
                case Z_POSITION: printf("Zposition"); break;
                case X_ROTATION: printf("Xrotation"); break;
                case Y_ROTATION: printf("Yrotation"); break;
                case Z_ROTATION: printf("Zrotation"); break;
            }

            if (curr_channel < (num_channels - 1)) {
                printf(", ");
            }
        }
        printf("]\n");

        if (bvh_joint_has_end_site(joint)) {
            const struct bvh_Offset end_site = bvh_joint_get_end_site(joint);
            printf(
                "%sEnd site = (%f, %f, %f)\n",
                indent,
                end_site.offset_x,
                end_site.offset_y,
                end_site.offset_z);
        }
        free(indent);
    }

    const float frame_time = bvh_file_get_frame_time(&bvh);
    const size_t num_frames = bvh_file_get_num_frames(&bvh);
    const size_t num_channels = bvh_file_get_num_channels(&bvh);

    printf("Frame time: %f\n", frame_time);
    printf("Num frames: %zu\n", num_frames);
    printf("Num channels: %zu\n", num_channels);

    for (size_t frame = 0; frame < num_frames; frame++) {
        const float* channels = bvh_file_get_frame(&bvh, frame);
        for (size_t ch = 0; ch < num_channels; ch++) {
            printf("%f", channels[ch]);
            if (ch < num_channels - 1) {
                printf(" ");
            }
        }
        printf("\n");
    }

    if (bvh_file_destroy(&bvh) == 0) {
        fprintf(stderr, "Could not destroy bvh file\n");
        return EXIT_FAILURE;
    }

    return EXIT_SUCCESS;
}
