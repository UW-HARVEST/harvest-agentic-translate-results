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
// dag_lib.c
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <limits.h>
#include "dag_lib.h"

// Create a new empty graph
graph_t* create_graph(void) {
    graph_t *graph = malloc(sizeof(graph_t));
    if (!graph) {
        fprintf(stderr, "Error: Failed to allocate graph\n");
        return NULL;
    }
    
    graph->node_count = 0;
    for (int i = 0; i < MAX_NODES; i++) {
        graph->nodes[i] = NULL;
    }
    
    return graph;
}

// Add a node to the graph
node_t* add_node(graph_t *graph, const char *city_name) {
    if (!graph || !city_name) {
        fprintf(stderr, "Error: NULL parameter in add_node\n");
        return NULL;
    }
    
    if (graph->node_count >= MAX_NODES) {
        fprintf(stderr, "Error: Graph is full (max %d nodes)\n", MAX_NODES);
        return NULL;
    }
    
    // Check if node already exists
    for (int i = 0; i < graph->node_count; i++) {
        if (strcmp(graph->nodes[i]->city_name, city_name) == 0) {
            fprintf(stderr, "Error: Node '%s' already exists\n", city_name);
            return NULL;
        }
    }
    
    // Allocate new node
    node_t *node = malloc(sizeof(node_t));
    if (!node) {
        fprintf(stderr, "Error: Failed to allocate node\n");
        return NULL;
    }
    
    // Initialize node
    strncpy(node->city_name, city_name, MAX_CITY_NAME - 1);
    node->city_name[MAX_CITY_NAME - 1] = '\0';
    node->ref_count = 1;
    node->edge_count = 0;
    
    // Add to graph
    graph->nodes[graph->node_count++] = node;
    
    return node;
}

// Add an edge between two nodes
int add_edge(node_t *from, node_t *to, int distance) {
    if (!from || !to) {
        fprintf(stderr, "Error: NULL node in add_edge\n");
        return -1;
    }
    
    if (from->edge_count >= MAX_EDGES) {
        fprintf(stderr, "Error: Node '%s' has maximum edges\n", from->city_name);
        return -1;
    }
    
    if (distance < 0) {
        fprintf(stderr, "Error: Negative distance not allowed\n");
        return -1;
    }
    
    // Check for duplicate edge
    for (int i = 0; i < from->edge_count; i++) {
        if (from->edges[i].destination == to) {
            fprintf(stderr, "Error: Edge already exists\n");
            return -1;
        }
    }
    
    // Add edge
    from->edges[from->edge_count].destination = to;
    from->edges[from->edge_count].distance = distance;
    from->edge_count++;
    
    return 0;
}

// Delete a node (decrement ref count, free if 0)
void delete_node(node_t *node) {
    if (!node) {
        return;
    }
    
    node->ref_count--;
    
    if (node->ref_count == 0) {
        free(node);
    }
}

// Helper function to increment ref count recursively
static void increment_refs_recursive(node_t *node, node_t **visited, int *visited_count) {
    if (!node) {
        return;
    }
    
    // Check if already visited
    for (int i = 0; i < *visited_count; i++) {
        if (visited[i] == node) {
            return;
        }
    }
    
    // Mark as visited
    if (*visited_count < MAX_NODES) {
        visited[(*visited_count)++] = node;
    }
    
    // Increment ref count
    node->ref_count++;
    
    // Recursively process all connected nodes
    for (int i = 0; i < node->edge_count; i++) {
        increment_refs_recursive(node->edges[i].destination, visited, visited_count);
    }
}

// Create shallow copy of subsection (increments ref counts)
node_t* shallow_copy(node_t *start) {
    if (!start) {
        fprintf(stderr, "Error: NULL node in shallow_copy\n");
        return NULL;
    }
    
    // Track visited nodes to avoid cycles
    node_t *visited[MAX_NODES];
    int visited_count = 0;
    
    // Increment ref counts for all reachable nodes
    increment_refs_recursive(start, visited, &visited_count);
    
    return start;
}

// Helper structure for shortest path algorithm
typedef struct {
    node_t *node;
    int distance;
    node_t *previous;
    int visited;
} dijkstra_node_t;

// Find shortest path using Dijkstra's algorithm
node_t** find_shortest_path(node_t *start, node_t *end, int *path_length) {
    if (!start || !end || !path_length) {
        fprintf(stderr, "Error: NULL parameter in find_shortest_path\n");
        return NULL;
    }
    
    // Initialize Dijkstra state
    dijkstra_node_t state[MAX_NODES];
    int state_count = 0;
    
    // Add start node
    state[state_count].node = start;
    state[state_count].distance = 0;
    state[state_count].previous = NULL;
    state[state_count].visited = 0;
    state_count++;
    
    node_t *current = start;
    
    while (current) {
        // Find current node in state
        int current_idx = -1;
        for (int i = 0; i < state_count; i++) {
            if (state[i].node == current) {
                current_idx = i;
                break;
            }
        }
        
        if (current_idx == -1) {
            break;
        }
        
        state[current_idx].visited = 1;
        
        // Check if we reached the end
        if (current == end) {
            break;
        }
        
        // Update distances for neighbors
        for (int i = 0; i < current->edge_count; i++) {
            node_t *neighbor = current->edges[i].destination;
            int new_distance = state[current_idx].distance + current->edges[i].distance;
            
            // Find or add neighbor in state
            int neighbor_idx = -1;
            for (int j = 0; j < state_count; j++) {
                if (state[j].node == neighbor) {
                    neighbor_idx = j;
                    break;
                }
            }
            
            if (neighbor_idx == -1 && state_count < MAX_NODES) {
                // Add new neighbor
                neighbor_idx = state_count;
                state[state_count].node = neighbor;
                state[state_count].distance = INT_MAX;
                state[state_count].previous = NULL;
                state[state_count].visited = 0;
                state_count++;
            }
            
            if (neighbor_idx != -1 && new_distance < state[neighbor_idx].distance) {
                state[neighbor_idx].distance = new_distance;
                state[neighbor_idx].previous = current;
            }
        }
        
        // Find next unvisited node with minimum distance
        int min_distance = INT_MAX;
        current = NULL;
        for (int i = 0; i < state_count; i++) {
            if (!state[i].visited && state[i].distance < min_distance) {
                min_distance = state[i].distance;
                current = state[i].node;
            }
        }
    }
    
    // Find end node in state
    int end_idx = -1;
    for (int i = 0; i < state_count; i++) {
        if (state[i].node == end) {
            end_idx = i;
            break;
        }
    }
    
    if (end_idx == -1 || state[end_idx].distance == INT_MAX) {
        fprintf(stderr, "No path found\n");
        *path_length = 0;
        return NULL;
    }
    
    // Reconstruct path
    node_t *path[MAX_NODES];
    int count = 0;
    node_t *current_node = end;
    
    while (current_node) {
        path[count++] = current_node;
        
        // Find previous node
        int current_state_idx = -1;
        for (int i = 0; i < state_count; i++) {
            if (state[i].node == current_node) {
                current_state_idx = i;
                break;
            }
        }
        
        if (current_state_idx == -1) {
            break;
        }
        
        current_node = state[current_state_idx].previous;
    }
    
    // Reverse path
    node_t **result = malloc(sizeof(node_t*) * count);
    if (!result) {
        fprintf(stderr, "Error: Failed to allocate path\n");
        *path_length = 0;
        return NULL;
    }
    
    for (int i = 0; i < count; i++) {
        result[i] = path[count - 1 - i];
    }
    
    *path_length = count;
    return result;
}

// Get node by city name
node_t* get_node_by_name(graph_t *graph, const char *city_name) {
    if (!graph || !city_name) {
        return NULL;
    }
    
    for (int i = 0; i < graph->node_count; i++) {
        if (strcmp(graph->nodes[i]->city_name, city_name) == 0) {
            return graph->nodes[i];
        }
    }
    
    return NULL;
}

// Print node information
void print_node(node_t *node) {
    if (!node) {
        printf("NULL node\n");
        return;
    }
    
    printf("City: %s (ref_count: %d)\n", node->city_name, node->ref_count);
    printf("  Edges:\n");
    for (int i = 0; i < node->edge_count; i++) {
        printf("    -> %s (distance: %d)\n", 
               node->edges[i].destination->city_name,
               node->edges[i].distance);
    }
}

// Print entire graph
void print_graph(graph_t *graph) {
    if (!graph) {
        printf("NULL graph\n");
        return;
    }
    
    printf("Graph with %d nodes:\n", graph->node_count);
    for (int i = 0; i < graph->node_count; i++) {
        print_node(graph->nodes[i]);
    }
}

// Free the entire graph
void free_graph(graph_t *graph) {
    if (!graph) {
        return;
    }
    
    // Decrement ref count for all nodes
    for (int i = 0; i < graph->node_count; i++) {
        delete_node(graph->nodes[i]);
    }
    
    free(graph);
}
