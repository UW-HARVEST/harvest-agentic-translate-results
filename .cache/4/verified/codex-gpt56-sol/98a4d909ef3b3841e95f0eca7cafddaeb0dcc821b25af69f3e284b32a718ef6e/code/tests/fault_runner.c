#define _GNU_SOURCE
#include "scene.h"
#include "shape.h"
#include <dlfcn.h>
#include <stdio.h>
#include <string.h>

typedef void (*arm_fn)(size_t);

int main(int argc, char **argv) {
    if (argc < 3) {
        return 90;
    }
    void *library = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
    arm_fn arm = (arm_fn)dlsym(RTLD_DEFAULT, "arm_failure");
    if (!library || !arm) {
        return 91;
    }

    if (strcmp(argv[2], "shape-init") == 0) {
        void (*function)(void) = (void (*)(void))dlsym(library, "shape_manager_init");
        arm(sizeof(shape_t));
        function();
        return 92;
    }
    if (strcmp(argv[2], "scene-create") == 0) {
        scene_t *(*function)(const char *) =
            (scene_t *(*)(const char *))dlsym(library, "scene_create");
        arm(sizeof(scene_t));
        return function("name") == NULL ? 0 : 93;
    }
    if (strcmp(argv[2], "scene-load") == 0 && argc == 4) {
        scene_t *(*function)(const char *) =
            (scene_t *(*)(const char *))dlsym(library, "scene_load");
        arm(sizeof(scene_t));
        return function(argv[3]) == NULL ? 0 : 94;
    }
    return 95;
}
