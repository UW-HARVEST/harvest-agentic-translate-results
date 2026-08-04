/*
 * Copyright 2025 MIT Lincoln Laboratory
 * Permission is hereby granted, free of charge,
 * to any person obtaining a copy of this software
 * and associated documentation files (the "Software"),
 * to deal in the Software without restriction,
 * including without limitation the rights to use, copy,
 * modify, merge, publish, distribute, sublicense,
 * and/or sell copies of the Software,
 * and to permit persons to whom the Software is furnished to do so,
 * subject to the following conditions:
 * 
 * The above copyright notice and this permission notice
 * shall be included in all copies or substantial portions of the Software.
 * 
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
 * EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
 * THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
 * IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
 * FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
 * TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
 * OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
 */
// main.c
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "dag_lib.h"

#define MAX_INPUT 256

void print_menu(void) {
    printf("\n=== DAG City Route Manager ===\n");
    printf("1. Add city (node)\n");
    printf("2. Add route (edge)\n");
    printf("3. Show all cities\n");
    printf("4. Show city details\n");
    printf("5. Find shortest path\n");
    printf("6. Make shallow copy of subsection\n");
    printf("7. Delete node\n");
    printf("8. Exit\n");
    printf("Choice: ");
}

int main(void) {
    graph_t *graph = create_graph();
    if (!graph) {
        fprintf(stderr, "Failed to create graph\n");
        return 1;
    }
    
    char input[MAX_INPUT];
    int choice;
    
    printf("City Route Management System\n");
    printf("Commands are read from stdin\n");
    
    while (1) {
        print_menu();
        
        if (fgets(input, MAX_INPUT, stdin) == NULL) {
            break;
        }
        
        if (sscanf(input, "%d", &choice) != 1) {
            printf("Invalid input\n");
            continue;
        }
        
        switch (choice) {
            case 1: {
                // Add city
                printf("Enter city name: ");
                if (fgets(input, MAX_INPUT, stdin) == NULL) {
                    break;
                }
                
                // Remove newline
                input[strcspn(input, "\n")] = 0;
                
                node_t *node = add_node(graph, input);
                if (node) {
                    printf("Added city: %s\n", input);
                } else {
                    printf("Failed to add city\n");
                }
                break;
            }
            
            case 2: {
                // Add route
                char from_city[MAX_INPUT], to_city[MAX_INPUT];
                int distance;
                
                printf("Enter from city: ");
                if (fgets(from_city, MAX_INPUT, stdin) == NULL) {
                    break;
                }
                from_city[strcspn(from_city, "\n")] = 0;
                
                printf("Enter to city: ");
                if (fgets(to_city, MAX_INPUT, stdin) == NULL) {
                    break;
                }
                to_city[strcspn(to_city, "\n")] = 0;
                
                printf("Enter distance: ");
                if (fgets(input, MAX_INPUT, stdin) == NULL) {
                    break;
                }
                if (sscanf(input, "%d", &distance) != 1) {
                    printf("Invalid distance\n");
                    break;
                }
                
                node_t *from = get_node_by_name(graph, from_city);
                node_t *to = get_node_by_name(graph, to_city);
                
                if (!from) {
                    printf("City '%s' not found\n", from_city);
                    break;
                }
                if (!to) {
                    printf("City '%s' not found\n", to_city);
                    break;
                }
                
                if (add_edge(from, to, distance) == 0) {
                    printf("Added route: %s -> %s (distance: %d)\n", 
                           from_city, to_city, distance);
                } else {
                    printf("Failed to add route\n");
                }
                break;
            }
            
            case 3: {
                // Show all cities
                print_graph(graph);
                break;
            }
            
            case 4: {
                // Show city details
                printf("Enter city name: ");
                if (fgets(input, MAX_INPUT, stdin) == NULL) {
                    break;
                }
                input[strcspn(input, "\n")] = 0;
                
                node_t *node = get_node_by_name(graph, input);
                if (node) {
                    print_node(node);
                } else {
                    printf("City '%s' not found\n", input);
                }
                break;
            }
            
            case 5: {
                // Find shortest path
                char start_city[MAX_INPUT], end_city[MAX_INPUT];
                
                printf("Enter start city: ");
                if (fgets(start_city, MAX_INPUT, stdin) == NULL) {
                    break;
                }
                start_city[strcspn(start_city, "\n")] = 0;
                
                printf("Enter end city: ");
                if (fgets(end_city, MAX_INPUT, stdin) == NULL) {
                    break;
                }
                end_city[strcspn(end_city, "\n")] = 0;
                
                node_t *start = get_node_by_name(graph, start_city);
                node_t *end = get_node_by_name(graph, end_city);
                
                if (!start) {
                    printf("City '%s' not found\n", start_city);
                    break;
                }
                if (!end) {
                    printf("City '%s' not found\n", end_city);
                    break;
                }
                
                int path_length;
                node_t **path = find_shortest_path(start, end, &path_length);
                
                if (path) {
                    printf("Shortest path from %s to %s:\n", start_city, end_city);
                    for (int i = 0; i < path_length; i++) {
                        printf("  %d. %s\n", i + 1, path[i]->city_name);
                    }
                    free(path);
                } else {
                    printf("No path found\n");
                }
                break;
            }
            
            case 6: {
                // Make shallow copy
                printf("Enter start city for shallow copy: ");
                if (fgets(input, MAX_INPUT, stdin) == NULL) {
                    break;
                }
                input[strcspn(input, "\n")] = 0;
                
                node_t *node = get_node_by_name(graph, input);
                if (!node) {
                    printf("City '%s' not found\n", input);
                    break;
                }
                
                node_t *copy = shallow_copy(node);
                if (copy) {
                    printf("Created shallow copy starting from %s\n", input);
                    printf("Reference counts incremented for all reachable nodes\n");
                    print_node(copy);
                } else {
                    printf("Failed to create shallow copy\n");
                }
                break;
            }
            
            case 7: {
                // Delete node
                printf("Enter city name to delete: ");
                if (fgets(input, MAX_INPUT, stdin) == NULL) {
                    break;
                }
                input[strcspn(input, "\n")] = 0;
                
                node_t *node = get_node_by_name(graph, input);
                if (!node) {
                    printf("City '%s' not found\n", input);
                    break;
                }
                
                printf("Current ref count: %d\n", node->ref_count);
                delete_node(node);
                printf("Decremented reference count for %s\n", input);
                printf("Note: Node will be freed when ref count reaches 0\n");
                break;
            }
            
            case 8: {
                // Exit
                printf("Freeing graph and exiting...\n");
                free_graph(graph);
                return 0;
            }
            
            default:
                printf("Invalid choice\n");
                break;
        }
    }
    
    free_graph(graph);
    return 0;
}
