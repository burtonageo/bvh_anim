#include <stdio.h>
#include <stdlib.h>

#include "bvh_anim/bvh_anim.h"

int main(int argc, const char* argv[]) {
    FILE* f = fopen("./data/test_mocapbank.bvh", "r");
    if (f == NULL) {
        return EXIT_FAILURE;
    }

    static const struct bvh_AllocCallbacks ALLOCATOR_DEFAULT = {0};

    struct bvh_BvhFile bvh;
    if (!bvh_read(f, &bvh, ALLOCATOR_DEFAULT, ALLOCATOR_DEFAULT)) {
        fclose(f);
        return EXIT_FAILURE;
    }

    printf("Num joints = %zu\n", bvh.bvh_num_joints);
    for (size_t i = 0; i < bvh.bvh_num_joints; i++) {
        const struct bvh_Joint* joint = &bvh.bvh_joints[i];
        printf("\tJoint name = %s\n", joint->joint_name);
        printf("\tJoint depth = %zu\n", joint->joint_depth);
        printf("\tJoint parent = %zu\n", joint->joint_parent_index);

        printf(
            "\tJoint offset %f %f %f\n",
            joint->joint_offset.offset_x,
            joint->joint_offset.offset_y,
            joint->joint_offset.offset_z);

        for (size_t curr_channel = 0; curr_channel < joint->joint_num_channels; curr_channel++) {
            const struct bvh_Channel* channel = &joint->joint_channels[curr_channel];
            printf("\tChannel %zu: ", channel->channel_index);
            switch (channel->channel_type) {
                case X_POSITION: printf("Xposition\n"); break;
                case Y_POSITION: printf("Yposition\n"); break;
                case Z_POSITION: printf("Zposition\n"); break;
                case X_ROTATION: printf("Xrotation\n"); break;
                case Y_ROTATION: printf("Yrotation\n"); break;
                case Z_ROTATION: printf("Zrotation\n"); break;
            }
        }

        if (joint->joint_has_end_site) {
            printf(
                "\tEnd site %f %f %f\n",
                joint->joint_end_site.offset_x,
                joint->joint_end_site.offset_y,
                joint->joint_end_site.offset_z);
        }

        printf("Frame time: %f\n", bvh.bvh_frame_time);
        printf("Num frames: %zu\n", bvh.bvh_num_frames);
        printf("Num channels: %zu\n", bvh.bvh_num_channels);

        for (size_t frame = 0; frame < bvh.bvh_num_frames; frame++) {
            const float* channels = bvh_get_frame(&bvh, frame);
            for (size_t ch = 0; ch < bvh.bvh_num_channels; ch++) {
                printf("%f", channels[ch]);
                if (ch < bvh.bvh_num_channels - 1) {
                    printf(" ");
                }
            }
            printf("\n");
        }
    }

    int exit_val = bvh_destroy(&bvh) == 0 ? EXIT_FAILURE : EXIT_SUCCESS;
    fclose(f);

    return exit_val;
}
