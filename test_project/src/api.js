// API module for handling HTTP requests
const express = require('express');
const cors = require('cors');
const helmet = require('helmet');

/**
 * Main API class for handling server operations
 */
class APIServer {
    constructor(port = 3000) {
        this.port = port;
        this.app = express();
        this.setupMiddleware();
        this.setupRoutes();
    }

    /**
     * Setup middleware for the Express app
     */
    setupMiddleware() {
        this.app.use(cors());
        this.app.use(helmet());
        this.app.use(express.json());
        this.app.use(express.urlencoded({ extended: true }));
    }

    /**
     * Setup routes for the API
     */
    setupRoutes() {
        this.app.get('/health', this.healthCheck);
        this.app.get('/api/users', this.getUsers);
        this.app.post('/api/users', this.createUser);
        this.app.get('/api/users/:id', this.getUserById);
        this.app.put('/api/users/:id', this.updateUser);
        this.app.delete('/api/users/:id', this.deleteUser);
    }

    /**
     * Health check endpoint
     */
    healthCheck(req, res) {
        res.json({ status: 'OK', timestamp: new Date().toISOString() });
    }

    /**
     * Get all users
     */
    async getUsers(req, res) {
        try {
            const users = await UserService.getAllUsers();
            res.json(users);
        } catch (error) {
            res.status(500).json({ error: error.message });
        }
    }

    /**
     * Create a new user
     */
    async createUser(req, res) {
        try {
            const userData = req.body;
            const user = await UserService.createUser(userData);
            res.status(201).json(user);
        } catch (error) {
            res.status(400).json({ error: error.message });
        }
    }

    /**
     * Get user by ID
     */
    async getUserById(req, res) {
        try {
            const userId = parseInt(req.params.id);
            const user = await UserService.getUserById(userId);
            if (!user) {
                return res.status(404).json({ error: 'User not found' });
            }
            res.json(user);
        } catch (error) {
            res.status(500).json({ error: error.message });
        }
    }

    /**
     * Update user
     */
    async updateUser(req, res) {
        try {
            const userId = parseInt(req.params.id);
            const userData = req.body;
            const user = await UserService.updateUser(userId, userData);
            res.json(user);
        } catch (error) {
            res.status(400).json({ error: error.message });
        }
    }

    /**
     * Delete user
     */
    async deleteUser(req, res) {
        try {
            const userId = parseInt(req.params.id);
            await UserService.deleteUser(userId);
            res.status(204).send();
        } catch (error) {
            res.status(500).json({ error: error.message });
        }
    }

    /**
     * Start the server
     */
    start() {
        this.app.listen(this.port, () => {
            console.log(`API Server running on port ${this.port}`);
        });
    }
}

/**
 * User service class for business logic
 */
class UserService {
    static users = new Map();
    static nextId = 1;

    static async getAllUsers() {
        return Array.from(this.users.values());
    }

    static async createUser(userData) {
        const user = {
            id: this.nextId++,
            ...userData,
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString()
        };
        this.users.set(user.id, user);
        return user;
    }

    static async getUserById(id) {
        return this.users.get(id);
    }

    static async updateUser(id, userData) {
        const user = this.users.get(id);
        if (!user) {
            throw new Error('User not found');
        }
        const updatedUser = {
            ...user,
            ...userData,
            updatedAt: new Date().toISOString()
        };
        this.users.set(id, updatedUser);
        return updatedUser;
    }

    static async deleteUser(id) {
        if (!this.users.has(id)) {
            throw new Error('User not found');
        }
        this.users.delete(id);
    }
}

/**
 * Validation utilities
 */
const ValidationUtils = {
    validateEmail: function(email) {
        const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
        return emailRegex.test(email);
    },

    validateUsername: function(username) {
        return username && username.length >= 3 && username.length <= 50;
    },

    validatePassword: function(password) {
        return password && password.length >= 8;
    }
};

// Configuration constants
const API_CONFIG = {
    DEFAULT_PORT: 3000,
    MAX_REQUEST_SIZE: '10mb',
    CORS_ORIGINS: ['http://localhost:3000', 'http://localhost:3001'],
    JWT_SECRET: process.env.JWT_SECRET || 'default-secret'
};

// Error handler middleware
function errorHandler(error, req, res, next) {
    console.error('API Error:', error);
    res.status(500).json({
        error: 'Internal Server Error',
        message: process.env.NODE_ENV === 'development' ? error.message : undefined
    });
}

// Rate limiting middleware
function rateLimiter(windowMs = 15 * 60 * 1000, max = 100) {
    const requests = new Map();

    return function(req, res, next) {
        const key = req.ip;
        const now = Date.now();
        const windowStart = now - windowMs;

        if (!requests.has(key)) {
            requests.set(key, []);
        }

        const userRequests = requests.get(key);
        const validRequests = userRequests.filter(time => time > windowStart);

        if (validRequests.length >= max) {
            return res.status(429).json({ error: 'Too many requests' });
        }

        validRequests.push(now);
        requests.set(key, validRequests);
        next();
    };
}

// Export modules
module.exports = {
    APIServer,
    UserService,
    ValidationUtils,
    API_CONFIG,
    errorHandler,
    rateLimiter
};
