#include <stdio.h>
#include <stdlib.h>

#include "bvh_anim/bvh_anim.h"

int main(int argc, const char* argv[]) {
    (void)argc;
    (void)argv;

    FILE* bvh_file = fopen("./data/test_mocapbank.bvh", "r");
    if (bvh_file == NULL) {
        return EXIT_FAILURE;
    }

    struct bvh_BvhFile bvh;
    int result = bvh_read(bvh_file, &bvh);
    fclose(bvh_file);

    if (result == 0) {
        return EXIT_FAILURE;
    }

    printf("Num joints = %zu\n", bvh.bvh_num_joints);
    for (size_t i = 0; i < bvh.bvh_num_joints; i++) {
        const struct bvh_Joint* joint = &bvh.bvh_joints[i];

        char* indent = (char*)malloc(sizeof(char) * joint->joint_depth);

        printf("%sJoint name = %s\n", indent, joint->joint_name);
        printf("%sJoint depth = %zu\n", indent, joint->joint_depth);
        printf("%sJoint parent = %zu\n", indent, joint->joint_parent_index);

        printf(
            "%sJoint offset %f %f %f\n",
            indent,
            joint->joint_offset.offset_x,
            joint->joint_offset.offset_y,
            joint->joint_offset.offset_z);

        for (size_t curr_channel = 0; curr_channel < joint->joint_num_channels; curr_channel++) {
            const struct bvh_Channel* channel = &joint->joint_channels[curr_channel];
            printf("%sChannel %zu: ", indent, channel->channel_index);
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
                "%sEnd site %f %f %f\n",
                indent,
                joint->joint_end_site.offset_x,
                joint->joint_end_site.offset_y,
                joint->joint_end_site.offset_z);
        }

        free(indent);
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

    return (bvh_destroy(&bvh) == 0) ? EXIT_FAILURE : EXIT_SUCCESS;
}
